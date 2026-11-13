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
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingMailer(Arc<Mutex<Option<String>>>);

#[async_trait]
impl EmailSender for RecordingMailer {
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
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
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
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

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
        .insert_header((AUTHORIZATION, format!("Bearer {}", rotated.refresh_token)))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, revoke).await.status(),
        StatusCode::NO_CONTENT
    );

    let refresh_revoked = actix_test::TestRequest::post()
        .uri("/auth/token/refresh")
        .insert_header((AUTHORIZATION, format!("Bearer {}", rotated.refresh_token)))
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
