use actix_web::{
    get,
    http::{header, StatusCode},
    post, web, HttpRequest, HttpResponse,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod join_links;
pub use join_links::{
    create_join_link, decide_join_request, list_join_links, list_my_join_requests,
    list_pending_join_requests, request_join_access, revoke_join_link, CreateJoinLinkRequest,
    JoinLinkResponse, JoinLinksApplication, JoinRequestDecision, JoinRequestResponse,
};

use crate::auth::{AuthApplication, AuthError, EmailSender, Problem};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpaceMemberResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpaceInvitationResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    pub email: String,
    pub role: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

#[derive(Clone)]
pub struct MembersApplication {
    pool: PgPool,
    mailer: std::sync::Arc<dyn EmailSender>,
    invite_url: String,
}

impl MembersApplication {
    pub fn new(pool: PgPool, mailer: std::sync::Arc<dyn EmailSender>, public_url: &str) -> Self {
        let invite_url = public_url
            .trim_end_matches("/auth/complete")
            .trim_end_matches('/')
            .to_owned()
            + "/welcome";
        Self {
            pool,
            mailer,
            invite_url,
        }
    }

    async fn list_members(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<SpaceMemberResponse>, MemberError> {
        sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT users.id, email_identities.email_normalized, membership.role \
             FROM space_memberships membership \
             JOIN users ON users.id = membership.user_id \
             JOIN email_identities ON email_identities.user_id = users.id \
             WHERE membership.space_id = $1 \
               AND EXISTS (SELECT 1 FROM space_memberships \
                           WHERE space_id = $1 AND user_id = $2) \
             ORDER BY membership.created_at, email_identities.created_at LIMIT 500",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(user_id, email, role)| SpaceMemberResponse {
                    user_id,
                    email,
                    role,
                })
                .collect()
        })
        .map_err(|_| MemberError::Unavailable)
    }

    async fn list_pending_for_space(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<SpaceInvitationResponse>, MemberError> {
        self.require_owner(space_id, user_id).await?;
        sqlx::query_as::<_, InvitationRow>(
            "SELECT i.id, i.space_id, s.name AS space_name, \
                    i.email_normalized AS email, i.role, i.expires_at \
             FROM space_invitations i JOIN spaces s ON s.id = i.space_id \
             WHERE i.space_id = $1 AND i.status = 'pending' AND i.expires_at > now() \
             ORDER BY i.created_at DESC",
        )
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(|_| MemberError::Unavailable)
    }

    async fn list_pending_for_email(
        &self,
        email: &str,
    ) -> Result<Vec<SpaceInvitationResponse>, MemberError> {
        sqlx::query_as::<_, InvitationRow>(
            "SELECT i.id, i.space_id, s.name AS space_name, \
                    i.email_normalized AS email, i.role, i.expires_at \
             FROM space_invitations i JOIN spaces s ON s.id = i.space_id \
             WHERE i.email_normalized = $1 AND i.status = 'pending' AND i.expires_at > now() \
             ORDER BY i.created_at DESC",
        )
        .bind(email)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(|_| MemberError::Unavailable)
    }

    async fn create_invitation(
        &self,
        space_id: Uuid,
        inviter_id: Uuid,
        email: &str,
    ) -> Result<SpaceInvitationResponse, MemberError> {
        let email = normalize_email(email).ok_or(MemberError::Invalid)?;
        self.require_owner(space_id, inviter_id).await?;
        let space = sqlx::query_as::<_, (String, bool)>(
            "SELECT s.name, EXISTS(SELECT 1 FROM email_identities \
             JOIN space_memberships m ON m.user_id = email_identities.user_id \
             WHERE email_identities.email_normalized = $2 AND m.space_id = s.id) \
             FROM spaces s WHERE s.id = $1",
        )
        .bind(space_id)
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| MemberError::Unavailable)?
        .ok_or(MemberError::NotFound)?;
        if space.1 {
            return Err(MemberError::AlreadyMember);
        }

        let invitation_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::days(7);
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        sqlx::query(
            "UPDATE space_invitations SET status = 'expired' \
             WHERE space_id = $1 AND email_normalized = $2 \
               AND status = 'pending' AND expires_at <= now()",
        )
        .bind(space_id)
        .bind(&email)
        .execute(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        sqlx::query(
            "INSERT INTO space_invitations (id, space_id, invited_by, email_normalized, expires_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(invitation_id)
        .bind(space_id)
        .bind(inviter_id)
        .bind(&email)
        .bind(expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            if error.as_database_error().is_some_and(|db| db.code().as_deref() == Some("23505")) {
                MemberError::AlreadyInvited
            } else {
                MemberError::Unavailable
            }
        })?;
        transaction
            .commit()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        let row = sqlx::query_as::<_, InvitationRow>(
            "SELECT i.id, i.space_id, s.name AS space_name, i.email_normalized AS email, \
                    i.role, i.expires_at \
             FROM space_invitations i JOIN spaces s ON s.id = i.space_id WHERE i.id = $1",
        )
        .bind(invitation_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        let link = format!("{}?invitation={invitation_id}", self.invite_url);
        if self
            .mailer
            .send_space_invitation(&email, &space.0, &link)
            .await
            .is_err()
        {
            let _ = sqlx::query("DELETE FROM space_invitations WHERE id = $1")
                .bind(invitation_id)
                .execute(&self.pool)
                .await;
            return Err(MemberError::EmailDelivery);
        }
        Ok(row.into())
    }

    async fn accept_invitation(
        &self,
        invitation_id: Uuid,
        user_id: Uuid,
        email: &str,
    ) -> Result<SpaceInvitationResponse, MemberError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        let row = sqlx::query_as::<_, InvitationRow>(
            "SELECT i.id, i.space_id, s.name AS space_name, \
                    i.email_normalized AS email, i.role, i.expires_at \
             FROM space_invitations i JOIN spaces s ON s.id = i.space_id \
             WHERE i.id = $1 AND i.status = 'pending' AND i.expires_at > now() FOR UPDATE OF i",
        )
        .bind(invitation_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?
        .ok_or(MemberError::NotFound)?;
        if row.email != email {
            return Err(MemberError::Forbidden);
        }
        sqlx::query("INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'viewer') ON CONFLICT (space_id, user_id) DO NOTHING")
            .bind(row.space_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| MemberError::Unavailable)?;
        sqlx::query(
            "UPDATE space_invitations SET status = 'accepted', accepted_at = now() WHERE id = $1",
        )
        .bind(invitation_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        Ok(row.into())
    }

    async fn update_member_role(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        member_id: Uuid,
        role: &str,
    ) -> Result<SpaceMemberResponse, MemberError> {
        self.require_owner(space_id, actor_id).await?;
        if !matches!(role, "viewer" | "editor") {
            return Err(MemberError::InvalidRole);
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        let row = sqlx::query_as::<_, (Uuid, String, String)>(
            "UPDATE space_memberships m SET role = $3 \
             FROM email_identities e \
             WHERE m.space_id = $1 AND m.user_id = $2 AND m.role <> 'owner' AND m.role <> $3 \
               AND e.user_id = m.user_id \
             RETURNING m.user_id, e.email_normalized, m.role",
        )
        .bind(space_id)
        .bind(member_id)
        .bind(role)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        let Some(row) = row else {
            let target = sqlx::query_as::<_, (String, String)>(
                "SELECT m.role, e.email_normalized FROM space_memberships m \
                 JOIN email_identities e ON e.user_id = m.user_id \
                 WHERE m.space_id = $1 AND m.user_id = $2",
            )
            .bind(space_id)
            .bind(member_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| MemberError::Unavailable)?;
            if target
                .as_ref()
                .is_some_and(|(target_role, _)| target_role == "owner")
            {
                return Err(MemberError::OwnerProtected);
            }
            let Some((target_role, email)) = target else {
                return Err(MemberError::NotFound);
            };
            transaction
                .commit()
                .await
                .map_err(|_| MemberError::Unavailable)?;
            return Ok(SpaceMemberResponse {
                user_id: member_id,
                email,
                role: target_role,
            });
        };
        crate::notifications::enqueue_notification(
            &mut transaction,
            space_id,
            actor_id,
            row.0,
            &crate::notifications::NotificationContent {
                event_type: "membership_change",
                title: "Your Space role changed",
                body: &format!("Your access is now {}.", row.2),
                destination: "members",
            },
        )
        .await
        .map_err(|_| MemberError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        Ok(SpaceMemberResponse {
            user_id: row.0,
            email: row.1,
            role: row.2,
        })
    }

    async fn remove_member(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        member_id: Uuid,
    ) -> Result<(), MemberError> {
        self.require_owner(space_id, actor_id).await?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| MemberError::Unavailable)?;
        let result = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM space_memberships WHERE space_id = $1 AND user_id = $2 AND role <> 'owner' RETURNING user_id",
        )
        .bind(space_id)
        .bind(member_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        if let Some(removed_user_id) = result {
            crate::notifications::enqueue_notification(
                &mut transaction,
                space_id,
                actor_id,
                removed_user_id,
                &crate::notifications::NotificationContent {
                    event_type: "membership_change",
                    title: "You were removed from a Space",
                    body: "Your access to this Space has ended.",
                    destination: "members",
                },
            )
            .await
            .map_err(|_| MemberError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| MemberError::Unavailable)?;
            return Ok(());
        }
        let target_role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(member_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        match target_role.as_deref() {
            Some("owner") => Err(MemberError::OwnerProtected),
            _ => Err(MemberError::NotFound),
        }
    }

    async fn revoke_invitation(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        invitation_id: Uuid,
    ) -> Result<(), MemberError> {
        self.require_owner(space_id, actor_id).await?;
        let result = sqlx::query(
            "UPDATE space_invitations SET status = 'revoked' \
             WHERE id = $1 AND space_id = $2 AND status = 'pending' AND expires_at > now()",
        )
        .bind(invitation_id)
        .bind(space_id)
        .execute(&self.pool)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(MemberError::NotFound)
        }
    }

    async fn require_owner(&self, space_id: Uuid, user_id: Uuid) -> Result<(), MemberError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| MemberError::Unavailable)?;
        match role.as_deref() {
            Some("owner") => Ok(()),
            Some("editor" | "viewer") => Err(MemberError::NotOwner),
            _ => Err(MemberError::NotFound),
        }
    }
}

#[derive(sqlx::FromRow)]
struct InvitationRow {
    id: Uuid,
    space_id: Uuid,
    space_name: String,
    email: String,
    role: String,
    expires_at: chrono::DateTime<Utc>,
}

impl From<InvitationRow> for SpaceInvitationResponse {
    fn from(row: InvitationRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            space_name: row.space_name,
            email: row.email,
            role: row.role,
            expires_at: row.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug)]
enum MemberError {
    Invalid,
    Forbidden,
    NotOwner,
    NotFound,
    AlreadyMember,
    AlreadyInvited,
    InvalidRole,
    OwnerProtected,
    EmailDelivery,
    Unavailable,
}

fn normalize_email(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 320 || value.chars().any(char::is_whitespace) {
        return None;
    }
    value
        .parse::<lettre::Address>()
        .ok()
        .map(|email| email.to_string().to_ascii_lowercase())
}

enum AuthenticationFailure {
    Unauthorized,
    Unavailable,
}

fn authentication_error(error: AuthenticationFailure) -> HttpResponse {
    match error {
        AuthenticationFailure::Unauthorized => problem(
            StatusCode::UNAUTHORIZED,
            "Authentication failed",
            "authentication_failed",
        ),
        AuthenticationFailure::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

async fn authenticated_account(
    auth: &AuthApplication,
    request: &HttpRequest,
) -> Result<crate::auth::Account, AuthenticationFailure> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty());
    let Some(token) = token else {
        return Err(AuthenticationFailure::Unauthorized);
    };
    auth.current_account(token)
        .await
        .map_err(|error| match error {
            AuthError::Unauthorized => AuthenticationFailure::Unauthorized,
            _ => AuthenticationFailure::Unavailable,
        })
}

fn problem(status: StatusCode, title: &'static str, code: &'static str) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title,
            status: status.as_u16(),
            code,
        })
}

fn error_response(error: MemberError) -> HttpResponse {
    match error {
        MemberError::Invalid => problem(
            StatusCode::BAD_REQUEST,
            "Invalid invitation",
            "invalid_invitation",
        ),
        MemberError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "Invitation is for a different email",
            "invitation_email_mismatch",
        ),
        MemberError::NotOwner => problem(
            StatusCode::FORBIDDEN,
            "Only the Space Owner can manage membership",
            "owner_required",
        ),
        MemberError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "Space or invitation not found",
            "membership_resource_not_found",
        ),
        MemberError::AlreadyMember => problem(
            StatusCode::CONFLICT,
            "Email already belongs to this Space",
            "already_a_member",
        ),
        MemberError::AlreadyInvited => problem(
            StatusCode::CONFLICT,
            "A pending invitation already exists",
            "already_invited",
        ),
        MemberError::InvalidRole => problem(
            StatusCode::BAD_REQUEST,
            "Invalid Member role",
            "invalid_member_role",
        ),
        MemberError::OwnerProtected => problem(
            StatusCode::CONFLICT,
            "Owners cannot be changed or removed in this workflow",
            "owner_membership_protected",
        ),
        MemberError::EmailDelivery => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Invitation email could not be sent",
            "email_delivery_failed",
        ),
        MemberError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(patch, path = "/spaces/{space_id}/members/{user_id}", params(("space_id" = Uuid, Path), ("user_id" = Uuid, Path)), request_body = UpdateMemberRoleRequest, security(("accessBearer" = [])), responses((status = 200, body = SpaceMemberResponse), (status = 400, body = Problem), (status = 403, body = Problem), (status = 404, body = Problem), (status = 409, body = Problem)))]
#[actix_web::patch("/spaces/{space_id}/members/{user_id}")]
pub async fn update_member_role(
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
    body: web::Json<UpdateMemberRoleRequest>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    let (space_id, member_id) = path.into_inner();
    match members
        .update_member_role(space_id, account.id, member_id, &body.role)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(delete, path = "/spaces/{space_id}/members/{user_id}", params(("space_id" = Uuid, Path), ("user_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem), (status = 409, body = Problem)))]
#[actix_web::delete("/spaces/{space_id}/members/{user_id}")]
pub async fn remove_member(
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    let (space_id, member_id) = path.into_inner();
    match members.remove_member(space_id, account.id, member_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(delete, path = "/spaces/{space_id}/invitations/{invitation_id}", params(("space_id" = Uuid, Path), ("invitation_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
#[actix_web::delete("/spaces/{space_id}/invitations/{invitation_id}")]
pub async fn revoke_invitation(
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    let (space_id, invitation_id) = path.into_inner();
    match members
        .revoke_invitation(space_id, account.id, invitation_id)
        .await
    {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(get, path = "/spaces/{space_id}/members", params(("space_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 200, body = [SpaceMemberResponse]), (status = 401, body = Problem), (status = 404, body = Problem)))]
#[get("/spaces/{space_id}/members")]
pub async fn list_members(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    match members.list_members(path.into_inner(), account.id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(get, path = "/spaces/{space_id}/invitations", params(("space_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 200, body = [SpaceInvitationResponse]), (status = 401, body = Problem), (status = 403, body = Problem)))]
#[get("/spaces/{space_id}/invitations")]
pub async fn list_space_invitations(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    match members
        .list_pending_for_space(path.into_inner(), account.id)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(post, path = "/spaces/{space_id}/invitations", params(("space_id" = Uuid, Path)), request_body = CreateInvitationRequest, security(("accessBearer" = [])), responses((status = 201, body = SpaceInvitationResponse), (status = 400, body = Problem), (status = 403, body = Problem), (status = 409, body = Problem)))]
#[post("/spaces/{space_id}/invitations")]
pub async fn create_invitation(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
    body: web::Json<CreateInvitationRequest>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    match members
        .create_invitation(path.into_inner(), account.id, &body.email)
        .await
    {
        Ok(result) => HttpResponse::Created().json(result),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(get, path = "/invitations/current", security(("accessBearer" = [])), responses((status = 200, body = [SpaceInvitationResponse]), (status = 401, body = Problem)))]
#[get("/invitations/current")]
pub async fn list_my_invitations(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    match members.list_pending_for_email(&account.email).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(post, path = "/invitations/{invitation_id}/accept", params(("invitation_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 200, body = SpaceInvitationResponse), (status = 403, body = Problem), (status = 404, body = Problem)))]
#[post("/invitations/{invitation_id}/accept")]
pub async fn accept_invitation(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    members: web::Data<MembersApplication>,
) -> HttpResponse {
    let account = match authenticated_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return authentication_error(error),
    };
    match members
        .accept_invitation(path.into_inner(), account.id, &account.email)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => error_response(error),
    }
}
