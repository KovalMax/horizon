use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    spaces::{
        self, CreateSpaceRequest, DeletedSpaceResponse, SpaceResponse, TransferOwnershipRequest,
    },
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
                    email: email.to_owned(),
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
                .find(|(recipient, _)| recipient == email)
                .map(|(_, link)| link.clone())
                .expect("mailbox should contain a sign-in link");
            let request = actix_test::TestRequest::post()
                .uri("/auth/magic-links/redeem")
                .set_json(RedeemMagicLinkRequest {
                    token: link.split_once("#token=").unwrap().1.to_owned(),
                })
                .to_request();
            let response = actix_test::call_service($app, request).await;
            assert_eq!(response.status(), StatusCode::OK);
            let authentication: auth::AuthenticationResponse =
                actix_test::read_body_json(response).await;
            authentication.tokens.access_token
        }
    }};
}

macro_rules! create_shared_space_request {
    ($token:expr, $name:expr) => {
        actix_test::TestRequest::post()
            .uri("/spaces")
            .insert_header((AUTHORIZATION, format!("Bearer {}", $token)))
            .set_json(CreateSpaceRequest {
                name: ($name).to_owned(),
            })
            .to_request()
    };
}

#[actix_web::test]
async fn ownership_transfer_deletion_restoration_and_expiry_follow_lifecycle_policy() {
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
            jwt_secret: "test-only-space-lifecycle-secret-32bytes".to_owned(),
            issuer: "horizon-test".to_owned(),
            audience: "horizon-api-test".to_owned(),
            key_id: "test-v1".to_owned(),
            public_url: "https://horizon.localhost/auth/complete".to_owned(),
        },
    )
    .expect("test auth configuration should be valid");
    let spaces_app = web::Data::new(spaces::SpacesApplication::new(pool.clone()));
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(auth_state))
            .app_data(spaces_app.clone())
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(auth::current_account)
            .service(spaces::list_spaces)
            .service(spaces::create_shared_space)
            .service(spaces::transfer_space_ownership)
            .service(spaces::delete_space)
            .service(spaces::list_deleted_spaces)
            .service(spaces::restore_space),
    )
    .await;

    let owner_email = format!("owner-{}@example.test", Uuid::new_v4());
    let member_email = format!("member-{}@example.test", Uuid::new_v4());
    let requester_email = format!("requester-{}@example.test", Uuid::new_v4());
    let owner_token = authenticate!(&app, &mailbox, &owner_email).await;
    let member_token = authenticate!(&app, &mailbox, &member_email).await;
    let _requester_token = authenticate!(&app, &mailbox, &requester_email).await;
    let owner_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&owner_email)
            .fetch_one(&pool)
            .await
            .unwrap();
    let member_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&member_email)
            .fetch_one(&pool)
            .await
            .unwrap();
    let requester_id: Uuid =
        sqlx::query_scalar("SELECT user_id FROM email_identities WHERE email_normalized = $1")
            .bind(&requester_email)
            .fetch_one(&pool)
            .await
            .unwrap();

    let request = create_shared_space_request!(&owner_token, "Lifecycle household");
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;
    let personal_space_id: Uuid =
        sqlx::query_scalar("SELECT id FROM spaces WHERE personal_owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{personal_space_id}/owner"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(TransferOwnershipRequest {
            new_owner_id: member_id,
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CONFLICT
    );
    sqlx::query(
        "INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(space.id)
    .bind(member_id)
    .execute(&pool)
    .await
    .expect("test member should join the Space");

    let mut quota_space_id = Uuid::nil();
    for number in 2..=10 {
        let response = actix_test::call_service(
            &app,
            create_shared_space_request!(&owner_token, format!("Quota Space {number}")),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let created: SpaceResponse = actix_test::read_body_json(response).await;
        quota_space_id = created.id;
    }
    sqlx::query(
        "INSERT INTO space_memberships (space_id, user_id, role) \
         SELECT owner_membership.space_id, $2, 'viewer' \
         FROM space_memberships owner_membership \
         JOIN spaces ON spaces.id = owner_membership.space_id \
         WHERE owner_membership.user_id = $1 AND owner_membership.role = 'owner' \
           AND spaces.kind = 'shared' AND spaces.deleted_at IS NULL \
         ON CONFLICT (space_id, user_id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(member_id)
    .execute(&pool)
    .await
    .expect("member should join every active owner Space");
    let mut member_owned_space_ids = Vec::new();
    for number in 1..=10 {
        let response = actix_test::call_service(
            &app,
            create_shared_space_request!(&member_token, format!("Member Space {number}")),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let created: SpaceResponse = actix_test::read_body_json(response).await;
        member_owned_space_ids.push(created.id);
    }
    let over_quota = actix_test::call_service(
        &app,
        create_shared_space_request!(&owner_token, "Over quota"),
    )
    .await;
    assert_eq!(over_quota.status(), StatusCode::CONFLICT);
    let problem: serde_json::Value = actix_test::read_body_json(over_quota).await;
    assert_eq!(problem["code"], "space_limit_reached");

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{quota_space_id}"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    let (first_create, second_create) = tokio::join!(
        actix_test::call_service(
            &app,
            create_shared_space_request!(&owner_token, "Concurrent Space A"),
        ),
        actix_test::call_service(
            &app,
            create_shared_space_request!(&owner_token, "Concurrent Space B"),
        ),
    );
    let (created_response, rejected_response) = if first_create.status() == StatusCode::CREATED {
        (first_create, second_create)
    } else {
        (second_create, first_create)
    };
    assert_eq!(created_response.status(), StatusCode::CREATED);
    assert_eq!(rejected_response.status(), StatusCode::CONFLICT);
    let concurrent_space: SpaceResponse = actix_test::read_body_json(created_response).await;

    let restore_at_limit = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{quota_space_id}/restore"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    let restore_at_limit = actix_test::call_service(&app, restore_at_limit).await;
    assert_eq!(restore_at_limit.status(), StatusCode::CONFLICT);
    let problem: serde_json::Value = actix_test::read_body_json(restore_at_limit).await;
    assert_eq!(problem["code"], "space_limit_reached");

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}", concurrent_space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let restore_after_delete = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{quota_space_id}/restore"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, restore_after_delete)
            .await
            .status(),
        StatusCode::OK
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{quota_space_id}"))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    sqlx::query(
        "UPDATE spaces SET deleted_at = now() - interval '8 days' \
         WHERE id IN ($1, $2)",
    )
    .bind(quota_space_id)
    .bind(concurrent_space.id)
    .execute(&pool)
    .await
    .expect("quota test Spaces should age beyond the recovery window");
    assert_eq!(spaces_app.purge_expired_deleted_spaces().await.unwrap(), 2);

    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/owner", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(TransferOwnershipRequest {
            new_owner_id: member_id,
        })
        .to_request();
    let transfer_at_limit = actix_test::call_service(&app, request).await;
    assert_eq!(transfer_at_limit.status(), StatusCode::CONFLICT);
    let problem: serde_json::Value = actix_test::read_body_json(transfer_at_limit).await;
    assert_eq!(problem["code"], "space_limit_reached");
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}", member_owned_space_ids[0]))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/owner", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(TransferOwnershipRequest {
            new_owner_id: member_id,
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let ownership: spaces::TransferOwnershipResponse = actix_test::read_body_json(response).await;
    assert_eq!(ownership.previous_owner_id, owner_id);
    assert_eq!(ownership.new_owner_id, member_id);
    let roles = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT user_id, role FROM space_memberships WHERE space_id = $1",
    )
    .bind(space.id)
    .fetch_all(&pool)
    .await
    .expect("membership roles should be queryable");
    assert!(roles
        .iter()
        .any(|(id, role)| *id == owner_id && role == "editor"));
    assert!(roles
        .iter()
        .any(|(id, role)| *id == member_id && role == "owner"));
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let link_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO space_join_links (id, space_id, created_by, role) VALUES ($1, $2, $3, 'viewer')",
    )
    .bind(link_id)
    .bind(space.id)
    .bind(member_id)
    .execute(&pool)
    .await
    .expect("test join link should be created");
    sqlx::query(
        "INSERT INTO space_join_requests (id, link_id, space_id, requester_id, requested_role) \
         VALUES ($1, $2, $3, $4, 'viewer')",
    )
    .bind(Uuid::new_v4())
    .bind(link_id)
    .bind(space.id)
    .bind(requester_id)
    .execute(&pool)
    .await
    .expect("test join request should be created");
    sqlx::query(
        "INSERT INTO space_invitations \
         (id, space_id, invited_by, email_normalized, status, created_at, expires_at) \
         VALUES ($1, $2, $3, $4, 'pending', now(), now() + interval '7 days')",
    )
    .bind(Uuid::new_v4())
    .bind(space.id)
    .bind(member_id)
    .bind(format!("pending-{}@example.test", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("test invitation should be created");

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let deleted: DeletedSpaceResponse = actix_test::read_body_json(response).await;
    assert_eq!(deleted.id, space.id);
    assert_eq!(
        deleted.restore_until,
        (chrono::DateTime::parse_from_rfc3339(&deleted.deleted_at).unwrap()
            + chrono::Duration::days(7))
        .to_rfc3339()
    );

    let request = actix_test::TestRequest::get()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let spaces: Vec<SpaceResponse> =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert!(!spaces.iter().any(|item| item.id == space.id));
    let invitation_status: String =
        sqlx::query_scalar("SELECT status FROM space_invitations WHERE space_id = $1")
            .bind(space.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let link_revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM space_join_links WHERE id = $1")
            .bind(link_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let pending_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM space_join_requests WHERE space_id = $1 AND status = 'pending'",
    )
    .bind(space.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(invitation_status, "revoked");
    assert!(link_revoked);
    assert_eq!(pending_requests, 1);

    let request = actix_test::TestRequest::get()
        .uri("/spaces/deleted")
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    let deleted_for_former_owner: Vec<DeletedSpaceResponse> =
        actix_test::read_body_json(actix_test::call_service(&app, request).await).await;
    assert!(deleted_for_former_owner.is_empty());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/restore", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/restore", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let restored: SpaceResponse = actix_test::read_body_json(response).await;
    assert_eq!(restored.role, "owner");
    assert_eq!(restored.member_count, 2);
    let restored_roles = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT user_id, role FROM space_memberships WHERE space_id = $1",
    )
    .bind(space.id)
    .fetch_all(&pool)
    .await
    .expect("restored memberships should be queryable");
    assert!(restored_roles
        .iter()
        .any(|(id, role)| *id == owner_id && role == "editor"));
    assert!(restored_roles
        .iter()
        .any(|(id, role)| *id == member_id && role == "owner"));
    let pending_after_restore: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM space_join_requests WHERE space_id = $1 AND status = 'pending'",
    )
    .bind(space.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending_after_restore, 1);

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    sqlx::query("UPDATE spaces SET deleted_at = now() - interval '7 days' WHERE id = $1")
        .bind(space.id)
        .execute(&pool)
        .await
        .expect("test deletion should age beyond the recovery window");
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/restore", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(spaces_app.purge_expired_deleted_spaces().await.unwrap(), 1);
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM spaces WHERE id = $1)")
        .bind(space.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!exists);
}
