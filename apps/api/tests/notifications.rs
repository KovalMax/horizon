use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    events::{self, CreateEventRequest, EventResponse},
    members::{
        self, JoinLinksApplication, JoinRequestDecision, JoinRequestResponse, MembersApplication,
        UpdateMemberRoleRequest,
    },
    notifications::{
        self, NotificationPageResponse, NotificationSettingsResponse,
        UpdateNotificationPreferencesRequest, UpdateNotificationReadRequest,
    },
    spaces::{self, CreateSpaceRequest, SpaceResponse},
    todos::{self, CreateTaskRequest},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct Mailbox(Arc<Mutex<Vec<(String, String)>>>);

#[async_trait]
impl EmailSender for Mailbox {
    async fn send_magic_link(&self, recipient: &str, link: &str) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push((recipient.to_owned(), link.to_owned()));
        Ok(())
    }

    async fn send_space_invitation(
        &self,
        recipient: &str,
        _space_name: &str,
        link: &str,
    ) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push((recipient.to_owned(), link.to_owned()));
        Ok(())
    }
}

macro_rules! authenticate {
    ($app:expr, $mailbox:expr, $email:expr) => {{
        async {
            let email = $email;
            let request = actix_test::TestRequest::post()
                .uri("/auth/magic-links")
                .set_json(MagicLinkRequest {
                    email: email.clone(),
                    time_zone: None,
                })
                .to_request();
            assert_eq!(
                actix_test::call_service($app, request).await.status(),
                StatusCode::ACCEPTED
            );
            let link = $mailbox
                .0
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|(recipient, _)| recipient == &email)
                .unwrap()
                .1
                .clone();
            let request = actix_test::TestRequest::post()
                .uri("/auth/magic-links/redeem")
                .set_json(RedeemMagicLinkRequest {
                    token: link.split_once("#token=").unwrap().1.to_owned(),
                })
                .to_request();
            let response = actix_test::call_service($app, request).await;
            let session: auth::AuthenticationResponse = actix_test::read_body_json(response).await;
            session.tokens.access_token
        }
    }};
}

#[actix_web::test]
async fn assigned_task_notifications_preferences_mute_and_read_state_are_persisted() {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect_lazy_with(horizon_api::database_connection_options());
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migrations should apply");
    let mailbox = Mailbox::default();
    let auth_state = auth::AuthApplication::with_email_sender(
        pool.clone(),
        Arc::new(Mailbox(mailbox.0.clone())),
        AuthSettings {
            jwt_secret: "test-only-notifications-secret-is-32-bytes".to_owned(),
            issuer: "horizon-test".to_owned(),
            audience: "horizon-api-test".to_owned(),
            key_id: "test-v1".to_owned(),
            public_url: "https://horizon.localhost/auth/complete".to_owned(),
        },
    )
    .expect("test auth configuration should be valid");
    let members_state = web::Data::new(MembersApplication::new(
        pool.clone(),
        auth_state.email_sender(),
        auth_state.public_url(),
    ));
    let join_links_state = web::Data::new(JoinLinksApplication::new(
        pool.clone(),
        auth_state.public_url(),
    ));
    let notifications_state =
        web::Data::new(notifications::NotificationsApplication::new(pool.clone()));
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .app_data(web::Data::new(todos::TodosApplication::new(pool.clone())))
            .app_data(web::Data::new(events::EventsApplication::new(pool.clone())))
            .app_data(members_state)
            .app_data(join_links_state)
            .app_data(notifications_state.clone())
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::create_shared_space)
            .service(todos::create_task)
            .service(events::create_event)
            .service(events::update_event)
            .service(members::update_member_role)
            .service(members::create_join_link)
            .service(members::request_join_access)
            .service(members::decide_join_request)
            .service(notifications::list_notifications)
            .service(notifications::mark_notification_read)
            .service(notifications::mark_all_notifications_read)
            .service(notifications::notification_preferences)
            .service(notifications::update_notification_preferences),
    )
    .await;
    let owner_email = format!("owner-{}@example.test", Uuid::new_v4());
    let member_email = format!("member-{}@example.test", Uuid::new_v4());
    let requester_email = format!("requester-{}@example.test", Uuid::new_v4());
    let owner_token = authenticate!(&app, &mailbox, owner_email.clone()).await;
    let member_token = authenticate!(&app, &mailbox, member_email.clone()).await;
    let requester_token = authenticate!(&app, &mailbox, requester_email.clone()).await;
    let owner_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&owner_email)
            .fetch_one(&pool)
            .await
            .unwrap();
    let requester_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&requester_email)
            .fetch_one(&pool)
            .await
            .unwrap();
    let member_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&member_email)
            .fetch_one(&pool)
            .await
            .unwrap();

    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateSpaceRequest {
            name: "Notifications Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;
    sqlx::query(
        "INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(space.id)
    .bind(member_id)
    .execute(&pool)
    .await
    .expect("notification recipient should be a Space Member");

    let create_task = |token: &str| {
        actix_test::TestRequest::post()
            .uri(&format!("/spaces/{}/tasks", space.id))
            .insert_header((AUTHORIZATION, format!("Bearer {token}")))
            .set_json(CreateTaskRequest {
                title: "Review insurance renewal".to_owned(),
                list_name: "Household".to_owned(),
                due_date: None,
                due_time: None,
                assignee_id: Some(member_id),
                reminder_offsets_minutes: vec![1440, 60],
            })
            .to_request()
    };
    let response = actix_test::call_service(&app, create_task(&owner_token)).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let request = actix_test::TestRequest::get()
        .uri("/notifications/preferences")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let settings: NotificationSettingsResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(settings.preferences.len(), 12);
    assert!(settings
        .preferences
        .iter()
        .filter(|item| item.channel == "in_app")
        .all(|item| item.enabled));
    assert!(settings
        .preferences
        .iter()
        .filter(|item| item.channel == "email")
        .all(|item| !item.enabled));

    let request = actix_test::TestRequest::get()
        .uri("/notifications?limit=1&offset=0")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(page.unread_count, 1);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].event_type, "task_assignment");
    assert_eq!(page.items[0].body, "Review insurance renewal");
    assert_eq!(page.items[0].space_id, space.id);
    let notification_id = page.items[0].id;

    let request = actix_test::TestRequest::get()
        .uri("/notifications")
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    let owner_page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert!(
        owner_page.items.is_empty(),
        "the actor should not receive their own notification"
    );

    let event_request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/events", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateEventRequest {
            title: "School concert".to_owned(),
            starts_at: "2026-12-10T18:00:00Z".to_owned(),
            ends_at: Some("2026-12-10T19:00:00Z".to_owned()),
            time_zone: "Europe/Berlin".to_owned(),
            all_day: false,
            location: None,
            notes: None,
            recurrence: None,
            recurrence_ends_on: None,
            my_reminder_offsets_minutes: None,
        })
        .to_request();
    let response = actix_test::call_service(&app, event_request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_event: EventResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::put()
        .uri(&format!("/events/{}", created_event.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateEventRequest {
            title: "School concert".to_owned(),
            starts_at: "2026-12-10T18:00:00Z".to_owned(),
            ends_at: Some("2026-12-10T19:00:00Z".to_owned()),
            time_zone: "Europe/Berlin".to_owned(),
            all_day: false,
            location: None,
            notes: None,
            recurrence: None,
            recurrence_ends_on: None,
            my_reminder_offsets_minutes: None,
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/members/{}", space.id, member_id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(UpdateMemberRoleRequest {
            role: "editor".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    let link_id = Uuid::new_v4();
    sqlx::query("INSERT INTO space_join_links (id, space_id, created_by, role) VALUES ($1, $2, $3, 'viewer')")
        .bind(link_id)
        .bind(space.id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{link_id}/requests"))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let join_request: JoinRequestResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{link_id}/requests"))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CREATED
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!(
            "/spaces/{}/join-requests/{}/decision",
            space.id, join_request.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(JoinRequestDecision {
            decision: "approved".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    for event_type in ["task_assignment", "event_change", "membership_change"] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND space_id = $2 AND event_type = $3",
        )
        .bind(member_id)
        .bind(space.id)
        .bind(event_type)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count, 1,
            "expected one {event_type} notification for affected Member"
        );
    }
    let owner_request_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND event_type = 'join_request'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let requester_result_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND event_type = 'join_request_result'",
    )
    .bind(requester_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owner_request_count, 1);
    assert_eq!(requester_result_count, 1);
    let event_change_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND space_id = $2 AND event_type = 'event_change'",
    )
    .bind(member_id)
    .bind(space.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        event_change_count, 1,
        "an unchanged event update should not duplicate notifications"
    );
    let request = actix_test::TestRequest::get()
        .uri("/notifications?limit=1&offset=0")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let first_page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.next_offset, Some(1));
    assert_eq!(first_page.unread_count, 3);
    let request = actix_test::TestRequest::get()
        .uri("/notifications?limit=1&offset=1")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let second_page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(second_page.items.len(), 1);
    assert_ne!(first_page.items[0].id, second_page.items[0].id);

    let preferences = UpdateNotificationPreferencesRequest {
        preferences: settings
            .preferences
            .iter()
            .map(|preference| notifications::NotificationPreferenceInput {
                event_type: preference.event_type.clone(),
                channel: preference.channel.clone(),
                enabled: preference.enabled,
            })
            .collect(),
        muted_space_ids: vec![space.id],
        default_reminder_time: "09:00".to_owned(),
        time_zone: "UTC".to_owned(),
    };
    let request = actix_test::TestRequest::put()
        .uri("/notifications/preferences")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .set_json(preferences)
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let before_muted_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND space_id = $2",
    )
    .bind(member_id)
    .bind(space.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        actix_test::call_service(&app, create_task(&owner_token))
            .await
            .status(),
        StatusCode::CREATED
    );
    let request = actix_test::TestRequest::get()
        .uri("/notifications")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(
        page.items.len() as i64,
        before_muted_count,
        "muted deliveries should not be backfilled"
    );

    let request = actix_test::TestRequest::put()
        .uri(&format!("/notifications/{notification_id}/read"))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .set_json(UpdateNotificationReadRequest { read: true })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let request = actix_test::TestRequest::put()
        .uri(&format!("/notifications/{notification_id}/read"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(UpdateNotificationReadRequest { read: true })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );
    let request = actix_test::TestRequest::get()
        .uri("/notifications")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(page.unread_count, 2);
    assert!(page
        .items
        .iter()
        .find(|item| item.id == notification_id)
        .unwrap()
        .read_at
        .is_some());

    let request = actix_test::TestRequest::post()
        .uri("/notifications/read-all")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let request = actix_test::TestRequest::get()
        .uri("/notifications")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let page: NotificationPageResponse =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert_eq!(page.unread_count, 0);
    sqlx::query("UPDATE notifications SET created_at = now() - interval '91 days' WHERE id = $1")
        .bind(notification_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(notifications_state.purge_expired().await.unwrap(), 1);
}
