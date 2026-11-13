use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    members::{self, CreateInvitationRequest, SpaceInvitationResponse, SpaceMemberResponse},
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
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(members_app))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::create_shared_space)
            .service(members::list_members)
            .service(members::list_space_invitations)
            .service(members::create_invitation)
            .service(members::list_my_invitations)
            .service(members::accept_invitation),
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

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/invitations", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {invitee_token}")))
        .set_json(CreateInvitationRequest {
            email: format!("other-{}@example.test", Uuid::new_v4()),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

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
}
