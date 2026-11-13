use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    places::{self, CreatePlaceRequest, PlacePageResponse, PlaceResponse},
    spaces::{self, CreateSpaceRequest, SpaceResponse},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

struct RecordingMailer(Arc<std::sync::Mutex<Option<String>>>);

#[async_trait]
impl EmailSender for RecordingMailer {
    async fn send_magic_link(&self, _recipient: &str, link: &str) -> Result<(), String> {
        *self.0.lock().expect("mailer lock should be available") = Some(link.to_owned());
        Ok(())
    }
}

#[actix_web::test]
async fn saved_places_are_space_scoped_and_viewers_cannot_create_them() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(horizon_api::database_connection_options());
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migrations should apply");
    let mailer = RecordingMailer(Arc::new(std::sync::Mutex::new(None)));
    let auth_state = auth::AuthState::with_email_sender(
        pool.clone(),
        Arc::new(RecordingMailer(mailer.0.clone())),
        AuthSettings {
            jwt_secret: "test-only-saved-places-key-is-32b".to_owned(),
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
            .app_data(web::Data::new(places::PlacesApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::create_shared_space)
            .service(places::list_places)
            .service(places::create_place),
    )
    .await;

    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email: format!("{}@example.test", Uuid::new_v4()),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::ACCEPTED
    );
    let link = mailer
        .0
        .lock()
        .expect("mailer lock should be available")
        .clone()
        .expect("magic link should have been sent");
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links/redeem")
        .set_json(RedeemMagicLinkRequest {
            token: link.split_once("#token=").unwrap().1.to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let session: auth::AuthenticationResponse = actix_test::read_body_json(response).await;
    let access_token = session.tokens.access_token;

    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateSpaceRequest {
            name: "Places test Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;

    let place_request = || CreatePlaceRequest {
        name: "Neighborhood bakery".to_owned(),
        category: "Food & drink".to_owned(),
        address: "12 Linden Street".to_owned(),
        note: Some("Best sourdough before noon".to_owned()),
    };
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/places", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(place_request())
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: PlaceResponse = actix_test::read_body_json(response).await;
    assert_eq!(created.space_id, space.id);
    assert_eq!(created.name, "Neighborhood bakery");
    assert_eq!(created.note.as_deref(), Some("Best sourdough before noon"));

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/places", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreatePlaceRequest {
            name: "Community garden".to_owned(),
            category: "Services".to_owned(),
            address: "1 Green Lane".to_owned(),
            note: None,
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let second_place: PlaceResponse = actix_test::read_body_json(response).await;

    let request = actix_test::TestRequest::get()
        .uri("/places?limit=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let places: PlacePageResponse = actix_test::read_body_json(response).await;
    assert_eq!(places.items.len(), 1);
    assert_eq!(places.next_offset, Some(1));
    let request = actix_test::TestRequest::get()
        .uri("/places?limit=1&offset=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let next_places: PlacePageResponse = actix_test::read_body_json(response).await;
    assert_eq!(next_places.items.len(), 1);
    assert_eq!(next_places.next_offset, None);
    assert_ne!(places.items[0].id, next_places.items[0].id);
    sqlx::query("DELETE FROM saved_places WHERE id = $1")
        .bind(second_place.id)
        .execute(&pool)
        .await
        .expect("test should clean up the additional place");

    sqlx::query("UPDATE space_memberships SET role = 'viewer' WHERE space_id = $1")
        .bind(space.id)
        .execute(&pool)
        .await
        .expect("test should be able to switch the member to Viewer");
    let request = actix_test::TestRequest::get()
        .uri("/places")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let viewer_places: PlacePageResponse = actix_test::read_body_json(response).await;
    assert_eq!(viewer_places.items.len(), 1);
    assert_eq!(viewer_places.items[0].id, created.id);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/places", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(place_request())
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let _ = sqlx::query("DELETE FROM spaces WHERE id = $1")
        .bind(space.id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(session.account.id)
        .execute(&pool)
        .await;
}
