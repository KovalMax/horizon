use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::auth::{
    self, AuthSettings, AuthenticationResult, EmailSender, MagicLinkRequest,
    RedeemMagicLinkRequest, TokenPair,
};
use horizon_api::spaces::{self, CreateSpaceRequest};
use sqlx::postgres::PgPoolOptions;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingMailer(Arc<Mutex<Option<String>>>, Arc<Mutex<Option<String>>>);

#[async_trait]
impl EmailSender for RecordingMailer {
    async fn send_sign_in(&self, recipient: &str, link: &str, code: &str) -> Result<(), String> {
        *self.1.lock().unwrap() = Some(code.to_owned());
        self.send_magic_link(recipient, link).await
    }

    async fn send_magic_link(&self, _recipient: &str, link: &str) -> Result<(), String> {
        *self.0.lock().expect("test mailer lock should be available") = Some(link.to_owned());
        Ok(())
    }
}

#[actix_web::test]
async fn magic_link_authentication_creates_rotates_and_revokes_a_session() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(horizon_api::database_connection_options());
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migrations should apply");

    let mailer = RecordingMailer::default();
    let auth_state = auth::AuthState::with_email_sender(
        pool.clone(),
        Arc::new(mailer.clone()),
        AuthSettings {
            jwt_secret: "test-only-authentication-key-32-bytes".to_owned(),
            issuer: "horizon-test".to_owned(),
            audience: "horizon-api-test".to_owned(),
            key_id: "test-v1".to_owned(),
            public_url: "https://horizon.localhost/auth/complete".to_owned(),
        },
    )
    .expect("test auth configuration should be valid");

    let app = actix_test::init_service(
        App::new()
            .app_data(auth::controllers::TrustedSignInProxy(
                "192.0.2.10".parse().unwrap(),
            ))
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(auth::redeem_sign_in_code)
            .service(auth::refresh_token)
            .service(auth::revoke_current_session)
            .service(auth::current_account)
            .service(spaces::list_spaces)
            .service(spaces::create_shared_space),
    )
    .await;

    let email = format!("{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email: format!(" {} ", email.to_ascii_uppercase()),
            time_zone: Some("Europe/Berlin".to_owned()),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let challenge: auth::MagicLinkAccepted = actix_test::read_body_json(response).await;

    // A wrong code consumes an attempt, but locking it does not invalidate the link.
    let correct = mailer.1.lock().unwrap().clone().unwrap();
    let wrong = if correct == "00000000" {
        "11111111"
    } else {
        "00000000"
    };
    for code in [wrong, wrong, wrong, wrong, wrong, correct.as_str()] {
        let response = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri("/auth/sign-in-codes/redeem")
                .set_json(
                    serde_json::json!({ "challenge_id": challenge.challenge_id, "code": code }),
                )
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let throttled = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email: email.clone(),
            time_zone: None,
        })
        .to_request();
    let response = actix_test::call_service(&app, throttled).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = response
        .headers()
        .get("Retry-After")
        .expect("rate-limit response should tell the client when to retry")
        .to_str()
        .expect("Retry-After should be valid text")
        .parse::<i64>()
        .expect("Retry-After should be a number of seconds");
    assert!((1..=30).contains(&retry_after));

    let link = mailer
        .0
        .lock()
        .expect("test mailer lock should be available")
        .clone()
        .expect("magic link should have been sent to the fake mailer");
    let secret = link
        .split_once("#token=")
        .expect("magic link should contain its token in the fragment")
        .1
        .to_owned();
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links/redeem")
        .set_json(RedeemMagicLinkRequest {
            token: secret.clone(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let list_spaces = actix_test::TestRequest::get().uri("/spaces").to_request();
    assert_eq!(
        actix_test::call_service(&app, list_spaces).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let authentication: AuthenticationResult = actix_test::read_body_json(response).await;
    assert_eq!(authentication.account.email, email);
    let time_zone = sqlx::query_scalar::<_, String>("SELECT time_zone FROM users WHERE id = $1")
        .bind(authentication.account.id)
        .fetch_one(&pool)
        .await
        .expect("new account should have a stored time zone");
    assert_eq!(time_zone, "Europe/Berlin");

    let rate_limit_ip = SocketAddr::new(IpAddr::V6(Ipv6Addr::from(Uuid::new_v4().as_u128())), 1234);
    for request_number in 0..20 {
        let request = actix_test::TestRequest::post()
            .uri("/auth/magic-links")
            .peer_addr(rate_limit_ip)
            .set_json(MagicLinkRequest {
                email: format!(
                    "rate-limit-{request_number}-{}@example.test",
                    Uuid::new_v4()
                ),
                time_zone: None,
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, request).await.status(),
            StatusCode::ACCEPTED
        );
    }

    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .peer_addr(rate_limit_ip)
        .set_json(MagicLinkRequest {
            email: format!("rate-limit-overflow-{}@example.test", Uuid::new_v4()),
            time_zone: None,
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = response
        .headers()
        .get("Retry-After")
        .expect("IP rate-limit response should provide a retry interval")
        .to_str()
        .expect("Retry-After should be valid text")
        .parse::<i64>()
        .expect("Retry-After should be a number of seconds");
    assert!((1..=60).contains(&retry_after));

    // Each request uses a different fixture address and checks the single
    // stored rate-limit key rather than exercising the public edge.
    for (peer, forwarded_for, expected) in [
        ("192.0.2.11:5000", "198.51.100.20", "192.0.2.11"),
        ("192.0.2.10:5000", "2001:db8::20", "2001:db8::20"),
        ("192.0.2.10:5000", "bad address", "192.0.2.10"),
        (
            "192.0.2.10:5000",
            "198.51.100.20, 198.51.100.21",
            "192.0.2.10",
        ),
    ] {
        let fixture_email = format!("proxy-{}@example.test", Uuid::new_v4());
        let response = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri("/auth/magic-links")
                .peer_addr(peer.parse().unwrap())
                .insert_header(("Forwarded", "for=203.0.113.7"))
                .insert_header(("X-Forwarded-For", forwarded_for))
                .set_json(MagicLinkRequest {
                    email: fixture_email.clone(),
                    time_zone: None,
                })
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let recorded: String = sqlx::query_scalar(
            "SELECT request_ip FROM auth_challenges WHERE email_normalized = $1",
        )
        .bind(fixture_email)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(recorded, expected);
    }

    let replay = actix_test::TestRequest::post()
        .uri("/auth/magic-links/redeem")
        .set_json(RedeemMagicLinkRequest { token: secret })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, replay).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let current_account = actix_test::TestRequest::get()
        .uri("/account/me")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", authentication.tokens.access_token),
        ))
        .to_request();
    let response = actix_test::call_service(&app, current_account).await;
    assert_eq!(response.status(), StatusCode::OK);

    let list_spaces = actix_test::TestRequest::get()
        .uri("/spaces")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", authentication.tokens.access_token),
        ))
        .to_request();
    let response = actix_test::call_service(&app, list_spaces).await;
    assert_eq!(response.status(), StatusCode::OK);
    let personal_spaces = String::from_utf8(actix_test::read_body(response).await.to_vec())
        .expect("space response should be JSON text");
    assert!(personal_spaces.contains("Personal"));

    let create_space = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", authentication.tokens.access_token),
        ))
        .set_json(CreateSpaceRequest {
            name: "  Family plans  ".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, create_space).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_space = String::from_utf8(actix_test::read_body(response).await.to_vec())
        .expect("created Space response should be JSON text");
    assert!(created_space.contains("Family plans"));
    assert!(created_space.contains("\"role\":\"owner\""));

    let invalid_space = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", authentication.tokens.access_token),
        ))
        .set_json(CreateSpaceRequest {
            name: "  ".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, invalid_space).await.status(),
        StatusCode::BAD_REQUEST
    );

    let old_refresh_token = authentication.tokens.refresh_token;
    let refresh = actix_test::TestRequest::post()
        .uri("/auth/token/refresh")
        .insert_header((AUTHORIZATION, format!("Bearer {old_refresh_token}")))
        .to_request();
    let response = actix_test::call_service(&app, refresh).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rotated: TokenPair = actix_test::read_body_json(response).await;
    assert_ne!(rotated.refresh_token, old_refresh_token);

    // Advance only this fixture's request timestamp past the resend cooldown.
    sqlx::query("UPDATE auth_challenges SET created_at = created_at - interval '1 minute' WHERE email_normalized = $1")
        .bind(&email)
        .execute(&pool)
        .await
        .unwrap();
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::post()
            .uri("/auth/magic-links")
            .set_json(MagicLinkRequest {
                email,
                time_zone: None,
            })
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let second_link = mailer.0.lock().unwrap().clone().unwrap();
    let second_secret = second_link.split_once("#token=").unwrap().1.to_owned();
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::post()
            .uri("/auth/magic-links/redeem")
            .set_json(RedeemMagicLinkRequest {
                token: second_secret,
            })
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let second_session: AuthenticationResult = actix_test::read_body_json(response).await;
    assert_eq!(second_session.account.id, authentication.account.id);

    let replay_refresh = actix_test::TestRequest::post()
        .uri("/auth/token/refresh")
        .insert_header((AUTHORIZATION, format!("Bearer {old_refresh_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, replay_refresh)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let revoke = actix_test::TestRequest::delete()
        .uri("/auth/session/current")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", second_session.tokens.refresh_token),
        ))
        .to_request();
    let protected_request = |token: &str| {
        actix_test::TestRequest::get()
            .uri("/spaces")
            .insert_header((AUTHORIZATION, format!("Bearer {token}")))
            .to_request()
    };
    assert_eq!(
        actix_test::call_service(&app, protected_request(&rotated.access_token))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        actix_test::call_service(&app, protected_request(&second_session.tokens.access_token))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        actix_test::call_service(&app, revoke).await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        actix_test::call_service(&app, protected_request(&second_session.tokens.access_token))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let refresh_revoked = actix_test::TestRequest::post()
        .uri("/auth/token/refresh")
        .insert_header((
            AUTHORIZATION,
            format!("Bearer {}", second_session.tokens.refresh_token),
        ))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, refresh_revoked)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(authentication.account.id)
        .execute(&pool)
        .await;
}
