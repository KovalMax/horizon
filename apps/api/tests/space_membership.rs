use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    members::{
        self, CreateInvitationRequest, SpaceInvitationResponse, SpaceMemberResponse,
        UpdateMemberRoleRequest,
    },
    spaces::{self, CreateSpaceRequest, SpaceResponse},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct Mailbox(Arc<Mutex<Vec<(String, String)>>>);

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
                .expect("mailbox should contain the sign-in link");
            let request = actix_test::TestRequest::post()
                .uri("/auth/magic-links/redeem")
                .set_json(RedeemMagicLinkRequest {
                    token: link.split_once("#token=").unwrap().1.to_owned(),
                })
                .to_request();
            let response = actix_test::call_service($app, request).await;
            assert_eq!(response.status(), StatusCode::OK);
            let session: auth::AuthenticationResponse = actix_test::read_body_json(response).await;
            session.tokens.access_token
        }
    }};
}

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

#[actix_web::test]
async fn email_invitation_is_verified_and_accepted_as_viewer() {
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
            jwt_secret: "test-only-space-membership-secret-32bytes".to_owned(),
            issuer: "horizon-test".to_owned(),
            audience: "horizon-api-test".to_owned(),
            key_id: "test-v1".to_owned(),
            public_url: "https://horizon.localhost/auth/complete".to_owned(),
        },
    )
    .expect("test auth configuration should be valid");
    let members_app = members::MembersApplication::new(
        pool.clone(),
        auth_state.email_sender(),
        auth_state.public_url(),
    );
    let join_links_app = members::JoinLinksApplication::new(pool.clone(), auth_state.public_url());
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(members_app))
            .app_data(web::Data::new(join_links_app))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::create_shared_space)
            .service(members::list_members)
            .service(members::update_member_role)
            .service(members::remove_member)
            .service(members::list_space_invitations)
            .service(members::create_invitation)
            .service(members::revoke_invitation)
            .service(members::list_my_invitations)
            .service(members::accept_invitation)
            .service(members::create_join_link)
            .service(members::list_join_links)
            .service(members::revoke_join_link)
            .service(members::request_join_access)
            .service(members::list_my_join_requests)
            .service(members::list_pending_join_requests)
            .service(members::decide_join_request),
    )
    .await;

    let owner_email = format!("owner-{}@example.test", Uuid::new_v4());
    let owner_token = authenticate!(&app, &mailbox, &owner_email).await;
    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateSpaceRequest {
            name: "Membership test Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;

    let invitee_email = format!("invitee-{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateInvitationRequest {
            email: invitee_email.clone(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let invitation: SpaceInvitationResponse = actix_test::read_body_json(response).await;
    assert_eq!(invitation.role, "viewer");
    assert_eq!(invitation.email, invitee_email);
    assert!(mailbox.0.lock().unwrap().iter().any(|(recipient, link)| {
        recipient == &invitation.email && link.contains("/welcome?invitation=")
    }));

    let invitee_token = authenticate!(&app, &mailbox, &invitation.email).await;
    let request = actix_test::TestRequest::get()
        .uri("/invitations/current")
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let invites: Vec<SpaceInvitationResponse> = actix_test::read_body_json(response).await;
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].id, invitation.id);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/invitations/{}/accept", invitation.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );

    let request = actix_test::TestRequest::get()
        .uri(&format!("/spaces/{}/members", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let space_members: Vec<SpaceMemberResponse> = actix_test::read_body_json(response).await;
    assert_eq!(space_members.len(), 2);
    assert!(space_members
        .iter()
        .any(|member| { member.email == invitation.email && member.role == "viewer" }));

    let invitee_id = space_members
        .iter()
        .find(|member| member.email == invitation.email)
        .unwrap()
        .user_id;
    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/members/{invitee_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(UpdateMemberRoleRequest {
            role: "editor".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let changed: SpaceMemberResponse = actix_test::read_body_json(response).await;
    assert_eq!(changed.role, "editor");

    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/members/{invitee_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .set_json(UpdateMemberRoleRequest {
            role: "viewer".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}/members/{invitee_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let owner_id = space_members
        .iter()
        .find(|member| member.role == "owner")
        .unwrap()
        .user_id;
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}/members/{owner_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CONFLICT
    );
    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/members/{owner_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(UpdateMemberRoleRequest {
            role: "viewer".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CONFLICT
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}/members/{invitee_id}", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let request = actix_test::TestRequest::get()
        .uri(&format!("/spaces/{}/members", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let remaining: Vec<SpaceMemberResponse> = actix_test::read_body_json(
        actix_test::call_service(
            &app,
            actix_test::TestRequest::get()
                .uri(&format!("/spaces/{}/members", space.id))
                .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
                .to_request(),
        )
        .await,
    )
    .await;
    assert_eq!(remaining.len(), 1);

    let other_owner_email = format!("other-owner-{}@example.test", Uuid::new_v4());
    let other_owner_token = authenticate!(&app, &mailbox, &other_owner_email).await;
    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {other_owner_token}")))
        .set_json(CreateSpaceRequest {
            name: "Separate membership test Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let other_space: SpaceResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::patch()
        .uri(&format!("/spaces/{}/members/{owner_id}", other_space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(UpdateMemberRoleRequest {
            role: "editor".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}/members/{owner_id}", other_space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!(
            "/spaces/{}/invitations/{}",
            other_space.id, invitation.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let expired_email = format!("expired-{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateInvitationRequest {
            email: expired_email.clone(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let expired_invitation: SpaceInvitationResponse = actix_test::read_body_json(response).await;
    sqlx::query(
        "UPDATE space_invitations \
         SET created_at = now() - interval '2 seconds', \
             expires_at = now() - interval '1 second' \
         WHERE id = $1",
    )
    .bind(expired_invitation.id)
    .execute(&pool)
    .await
    .expect("test should be able to expire the invitation");
    let expired_user_token = authenticate!(&app, &mailbox, &expired_email).await;
    let request = actix_test::TestRequest::get()
        .uri("/invitations/current")
        .insert_header((AUTHORIZATION, format!("Bearer {expired_user_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let pending: Vec<SpaceInvitationResponse> = actix_test::read_body_json(response).await;
    assert!(pending.is_empty());
    let request = actix_test::TestRequest::post()
        .uri(&format!("/invitations/{}/accept", expired_invitation.id))
        .insert_header((AUTHORIZATION, format!("Bearer {expired_user_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateInvitationRequest {
            email: expired_email.clone(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let replacement: SpaceInvitationResponse = actix_test::read_body_json(response).await;
    assert_eq!(replacement.email, expired_email);

    let request = actix_test::TestRequest::delete()
        .uri(&format!(
            "/spaces/{}/invitations/{}",
            space.id, replacement.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateInvitationRequest {
            email: expired_email.clone(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CREATED
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .set_json(CreateInvitationRequest {
            email: format!("other-{}@example.test", Uuid::new_v4()),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/join-links", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::CreateJoinLinkRequest {
            role: "editor".to_owned(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339()),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let join_link: members::JoinLinkResponse = actix_test::read_body_json(response).await;
    assert_eq!(join_link.role, "editor");
    assert!(join_link.url.contains(&format!("?join={}", join_link.id)));
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/join-links", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::CreateJoinLinkRequest {
            role: "owner".to_owned(),
            expires_at: None,
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::BAD_REQUEST
    );

    let requester_email = format!("requester-{}@example.test", Uuid::new_v4());
    let requester_token = authenticate!(&app, &mailbox, &requester_email).await;
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let join_request: members::JoinRequestResponse = actix_test::read_body_json(response).await;
    assert_eq!(join_request.status, "pending");
    assert_eq!(join_request.role, "editor");
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let duplicate_request: members::JoinRequestResponse =
        actix_test::read_body_json(response).await;
    assert_eq!(duplicate_request.id, join_request.id);

    let request = actix_test::TestRequest::get()
        .uri(&format!("/spaces/{}/join-requests", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let pending_requests: Vec<members::JoinRequestResponse> =
        actix_test::read_body_json(response).await;
    assert_eq!(pending_requests.len(), 1);
    assert_eq!(
        pending_requests[0].requester_email.as_deref(),
        Some(requester_email.as_str())
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!(
            "/spaces/{}/join-requests/{}/decision",
            space.id, join_request.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::JoinRequestDecision {
            decision: "approved".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let approved_request: members::JoinRequestResponse = actix_test::read_body_json(response).await;
    assert_eq!(approved_request.status, "approved");
    let request = actix_test::TestRequest::get()
        .uri("/join-requests/current")
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let requester_status: Vec<members::JoinRequestResponse> =
        actix_test::read_body_json(response).await;
    assert_eq!(requester_status[0].status, "approved");
    let request = actix_test::TestRequest::get()
        .uri(&format!("/spaces/{}/members", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let members_list: Vec<SpaceMemberResponse> = actix_test::read_body_json(response).await;
    assert!(members_list
        .iter()
        .any(|member| member.email == requester_email && member.role == "editor"));
    let request = actix_test::TestRequest::get()
        .uri(&format!("/spaces/{}/join-links", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {requester_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CONFLICT
    );

    let decline_email = format!("declined-{}@example.test", Uuid::new_v4());
    let decline_token = authenticate!(&app, &mailbox, &decline_email).await;
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {decline_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let first_request: members::JoinRequestResponse = actix_test::read_body_json(response).await;
    let request = actix_test::TestRequest::post()
        .uri(&format!(
            "/spaces/{}/join-requests/{}/decision",
            space.id, first_request.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::JoinRequestDecision {
            decision: "declined".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::OK
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {decline_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let retried_request: members::JoinRequestResponse = actix_test::read_body_json(response).await;
    assert_ne!(retried_request.id, first_request.id);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/join-links", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::CreateJoinLinkRequest {
            role: "viewer".to_owned(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339()),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let expiring_link: members::JoinLinkResponse = actix_test::read_body_json(response).await;
    sqlx::query(
        "UPDATE space_join_links SET created_at = now() - interval '2 seconds', \
         expires_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(expiring_link.id)
    .execute(&pool)
    .await
    .expect("test should be able to expire a join link");
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", expiring_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {decline_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let request = actix_test::TestRequest::delete()
        .uri(&format!("/spaces/{}/join-links/{}", space.id, join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/join-links/{}/requests", join_link.id))
        .insert_header((AUTHORIZATION, format!("Bearer {decline_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!(
            "/spaces/{}/join-requests/{}/decision",
            space.id, retried_request.id
        ))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(members::JoinRequestDecision {
            decision: "approved".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    sqlx::query(
        "UPDATE space_memberships SET role = 'editor' WHERE space_id = $1 AND role = 'owner'",
    )
    .bind(space.id)
    .execute(&pool)
    .await
    .expect("test should be able to switch Owner to Editor");
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {owner_token}")))
        .set_json(CreateInvitationRequest {
            email: format!("editor-{}@example.test", Uuid::new_v4()),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    let request = actix_test::TestRequest::post()
        .uri(&format!("/invitations/{}/accept", invitation.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND
    );

    let _ = sqlx::query("DELETE FROM spaces WHERE id = $1")
        .bind(space.id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM spaces WHERE id = $1")
        .bind(other_space.id)
        .execute(&pool)
        .await;
}
