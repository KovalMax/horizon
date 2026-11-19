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
            .service(events::list_occurrences)
            .service(events::create_event)
            .service(events::update_event)
            .service(events::update_occurrence)
            .service(events::cancel_occurrence)
            .service(events::delete_event),
    )
    .await;

    let email = format!("{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email: email.clone(),
            time_zone: None,
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
        recurrence_ends_on: None,
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

    let mut legacy_time_zone_request = create_request();
    legacy_time_zone_request.title = "Kyiv timezone alias".to_owned();
    legacy_time_zone_request.time_zone = "Europe/Kiev".to_owned();
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&legacy_time_zone_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let legacy_time_zone_event: EventResponse = actix_test::read_body_json(response).await;
    assert_eq!(legacy_time_zone_event.time_zone, "Europe/Kyiv");

    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-12-10&end_date=2026-12-10")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/events/{}", legacy_time_zone_event.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );

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

    let mut daily_request = create_request();
    daily_request.title = "Daily DST series".to_owned();
    daily_request.starts_at = "2026-03-27T08:00:00Z".to_owned();
    daily_request.ends_at = Some("2026-03-27T09:00:00Z".to_owned());
    daily_request.time_zone = "Europe/Berlin".to_owned();
    daily_request.recurrence = Some("daily".to_owned());
    daily_request.recurrence_ends_on = Some("2026-03-31".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&daily_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let daily_series: EventResponse = actix_test::read_body_json(response).await;

    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-27&end_date=2026-03-31&limit=20")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let daily_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(daily_occurrences.items.len(), 5);
    assert!(daily_occurrences.items[0].occurrence_id.is_some());
    assert_eq!(
        daily_occurrences.items[0].occurrence_date.as_deref(),
        Some("2026-03-27")
    );
    assert_eq!(
        daily_occurrences.items[0].starts_at,
        "2026-03-27T08:00:00+00:00"
    );
    assert_eq!(
        daily_occurrences.items[2].occurrence_date.as_deref(),
        Some("2026-03-29")
    );
    assert_eq!(
        daily_occurrences.items[2].starts_at,
        "2026-03-29T07:00:00+00:00"
    );

    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-27&end_date=2026-03-31&limit=2")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let first_range_page: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(first_range_page.items.len(), 2);
    assert_eq!(first_range_page.next_offset, Some(2));
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-27&end_date=2026-03-31&limit=2&offset=2")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let second_range_page: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(second_range_page.items.len(), 2);
    assert_eq!(second_range_page.next_offset, Some(4));
    assert_ne!(
        first_range_page.items[1].occurrence_id,
        second_range_page.items[0].occurrence_id
    );

    let mut occurrence_update = create_request();
    occurrence_update.title = "Moved one date".to_owned();
    occurrence_update.starts_at = "2026-03-28T10:30:00+01:00".to_owned();
    occurrence_update.ends_at = Some("2026-03-28T11:30:00+01:00".to_owned());
    occurrence_update.recurrence = None;
    let request = actix_test::TestRequest::put()
        .uri(&format!(
            "/events/{}/occurrences/2026-03-28",
            daily_series.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&occurrence_update)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let override_response: EventResponse = actix_test::read_body_json(response).await;
    assert!(override_response.is_override);

    let request = actix_test::TestRequest::delete()
        .uri(&format!(
            "/events/{}/occurrences/2026-03-30",
            daily_series.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );

    let mut series_update = daily_request.clone();
    series_update.title = "Renamed daily series".to_owned();
    series_update.recurrence_ends_on = Some("2026-03-29".to_owned());
    let request = actix_test::TestRequest::put()
        .uri(&format!("/events/{}", daily_series.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&series_update)
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-27&end_date=2026-03-31&limit=20")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let after_series_edit: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(after_series_edit.items.len(), 3);
    assert_eq!(after_series_edit.items[0].title, "Renamed daily series");
    assert_eq!(after_series_edit.items[1].title, "Moved one date");
    assert_eq!(
        after_series_edit.items[2].occurrence_date.as_deref(),
        Some("2026-03-29")
    );

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/events/{}", daily_series.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );

    let mut weekly_request = create_request();
    weekly_request.title = "Weekly series".to_owned();
    weekly_request.starts_at = "2026-03-27T18:00:00Z".to_owned();
    weekly_request.recurrence = Some("weekly".to_owned());
    weekly_request.recurrence_ends_on = Some("2026-04-10".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&weekly_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let weekly_series: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-27&end_date=2026-04-10&limit=20")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let weekly_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    let weekly_dates = weekly_occurrences
        .items
        .iter()
        .filter(|item| item.id == weekly_series.id)
        .map(|item| item.occurrence_date.as_deref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(weekly_dates, ["2026-03-27", "2026-04-03", "2026-04-10"]);

    let mut all_day_request = create_request();
    all_day_request.title = "All-day series".to_owned();
    all_day_request.starts_at = "2026-03-28T00:00:00Z".to_owned();
    all_day_request.ends_at = None;
    all_day_request.all_day = true;
    all_day_request.time_zone = "Pacific/Auckland".to_owned();
    all_day_request.recurrence = Some("daily".to_owned());
    all_day_request.recurrence_ends_on = Some("2026-03-29".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&all_day_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let all_day_series: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-28&end_date=2026-03-29&limit=20")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let all_day_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    let all_day_dates = all_day_occurrences
        .items
        .iter()
        .filter(|item| item.id == all_day_series.id)
        .collect::<Vec<_>>();
    assert_eq!(all_day_dates.len(), 2);
    assert!(all_day_dates.iter().all(|item| item.all_day));
    assert_eq!(
        all_day_dates[0].occurrence_date.as_deref(),
        Some("2026-03-28")
    );
    assert_eq!(all_day_dates[0].starts_at, "2026-03-27T11:00:00+00:00");

    let mut gap_series_request = create_request();
    gap_series_request.title = "DST gap series".to_owned();
    gap_series_request.starts_at = "2026-03-28T01:30:00Z".to_owned();
    gap_series_request.ends_at = Some("2026-03-28T03:30:00Z".to_owned());
    gap_series_request.recurrence = Some("daily".to_owned());
    gap_series_request.recurrence_ends_on = Some("2026-03-29".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&gap_series_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let gap_series: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-03-28&end_date=2026-03-29&limit=10")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let gap_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    let gap_after_transition = gap_occurrences
        .items
        .iter()
        .find(|item| {
            item.id == gap_series.id && item.occurrence_date.as_deref() == Some("2026-03-29")
        })
        .expect("the recurrence should include the day after the DST transition");
    assert_eq!(gap_after_transition.starts_at, "2026-03-29T01:30:00+00:00");

    let mut overlap_series_request = create_request();
    overlap_series_request.title = "DST overlap series".to_owned();
    overlap_series_request.starts_at = "2026-10-24T00:30:00Z".to_owned();
    overlap_series_request.ends_at = Some("2026-10-24T01:30:00Z".to_owned());
    overlap_series_request.recurrence = Some("daily".to_owned());
    overlap_series_request.recurrence_ends_on = Some("2026-10-25".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&overlap_series_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let overlap_series: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2026-10-24&end_date=2026-10-25&limit=10")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let overlap_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    let overlap_after_transition = overlap_occurrences
        .items
        .iter()
        .find(|item| {
            item.id == overlap_series.id && item.occurrence_date.as_deref() == Some("2026-10-25")
        })
        .expect("the recurrence should include the day after the DST transition");
    assert_eq!(
        overlap_after_transition.starts_at,
        "2026-10-25T00:30:00+00:00"
    );

    let mut monthly_request = create_request();
    monthly_request.title = "Month end series".to_owned();
    monthly_request.starts_at = "2027-01-31T18:00:00Z".to_owned();
    monthly_request.ends_at = None;
    monthly_request.time_zone = "UTC".to_owned();
    monthly_request.recurrence = Some("monthly".to_owned());
    monthly_request.recurrence_ends_on = Some("2027-03-31".to_owned());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(&monthly_request)
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let monthly_series: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::get()
        .uri("/events?start_date=2027-01-31&end_date=2027-03-31&limit=10")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let monthly_occurrences: EventPageResponse = actix_test::read_body_json(response).await;
    assert_eq!(monthly_occurrences.items.len(), 2);
    assert_eq!(
        monthly_occurrences.items[0].occurrence_date.as_deref(),
        Some("2027-01-31")
    );
    assert_eq!(
        monthly_occurrences.items[1].occurrence_date.as_deref(),
        Some("2027-03-31")
    );

    for event_id in [
        weekly_series.id,
        all_day_series.id,
        gap_series.id,
        overlap_series.id,
        monthly_series.id,
    ] {
        let request = actix_test::TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, request).await.status(),
            StatusCode::NO_CONTENT
        );
    }

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
