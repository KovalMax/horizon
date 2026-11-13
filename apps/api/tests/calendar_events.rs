use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    events::{self, CreateEventRequest, EventPageResponse, EventResponse},
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
async fn calendar_events_persist_with_membership_scoped_read_and_editor_writes() {
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
            jwt_secret: "test-only-calendar-event-key-32-bytes".to_owned(),
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
            .app_data(web::Data::new(events::EventsApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::list_spaces)
            .service(spaces::create_shared_space)
            .service(events::list_events)
            .service(events::create_event)
            .service(events::update_event)
            .service(events::delete_event),
    )
    .await;

    let email = format!("{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email: email.clone(),
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
    let token = link.split_once("#token=").unwrap().1.to_owned();
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links/redeem")
        .set_json(RedeemMagicLinkRequest { token })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let session: auth::AuthenticationResponse = actix_test::read_body_json(response).await;
    let access_token = session.tokens.access_token;

    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateSpaceRequest {
            name: "Calendar test Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;

    let request = actix_test::TestRequest::get().uri("/events").to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let create_request = || CreateEventRequest {
        title: "School concert".to_owned(),
        starts_at: "2026-12-10T18:00:00Z".to_owned(),
        ends_at: Some("2026-12-10T19:30:00Z".to_owned()),
        time_zone: "Europe/Berlin".to_owned(),
        all_day: false,
        location: Some("Town hall".to_owned()),
        notes: Some("Bring tickets".to_owned()),
        recurrence: None,
    };
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_request())
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: EventResponse = actix_test::read_body_json(response).await;
    assert_eq!(created.title, "School concert");
    assert_eq!(created.space_id, space.id);
    assert_eq!(created.time_zone, "Europe/Berlin");

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json({
            let mut event = create_request();
            event.title = "Second calendar event".to_owned();
            event.starts_at = "2026-12-11T18:00:00Z".to_owned();
            event.ends_at = Some("2026-12-11T19:30:00Z".to_owned());
            event
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let second_event: EventResponse = actix_test::read_body_json(response).await;

    let request = actix_test::TestRequest::get()
        .uri("/events?limit=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let events: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(events.items.len(), 1);
    assert_eq!(events.next_offset, Some(1));
    let request = actix_test::TestRequest::get()
        .uri("/events?limit=1&offset=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let next_events: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(next_events.items.len(), 1);
    assert_eq!(next_events.next_offset, None);
    assert_ne!(events.items[0].id, next_events.items[0].id);
    let request = actix_test::TestRequest::get()
        .uri("/events?limit=101")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("DELETE FROM calendar_events WHERE id = $1")
        .bind(second_event.id)
        .execute(&pool)
        .await
        .expect("test should clean up the additional event");

    sqlx::query("UPDATE space_memberships SET role = 'editor' WHERE space_id = $1")
        .bind(space.id)
        .execute(&pool)
        .await
        .expect("test should be able to switch the member to Editor");

    let mut update = create_request();
    update.title = "Concert moved".to_owned();
    let request = actix_test::TestRequest::put()
        .uri(&format!("/events/{}", created.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(update)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: EventResponse = actix_test::read_body_json(response).await;
    assert_eq!(updated.title, "Concert moved");

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/events/{}", created.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_request())
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let viewer_event: EventResponse = actix_test::read_body_json(response).await;

    sqlx::query("UPDATE space_memberships SET role = 'viewer' WHERE space_id = $1")
        .bind(space.id)
        .execute(&pool)
        .await
        .expect("test should be able to switch the member to Viewer");
    let request = actix_test::TestRequest::get()
        .uri("/events")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let viewer_events: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(viewer_events.items.len(), 1);
    assert_eq!(viewer_events.items[0].id, viewer_event.id);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_request())
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let request = actix_test::TestRequest::put()
        .uri(&format!("/events/{}", viewer_event.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_request())
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/events/{}", viewer_event.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(session.account.id)
        .execute(&pool)
        .await;
}
