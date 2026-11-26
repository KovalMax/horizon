use actix_web::{
    delete, get,
    http::{header, StatusCode},
    post, web, HttpRequest, HttpResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};

use super::{lock_active_space_owner, OwnerAuthorizationError};

#[derive(Clone)]
pub struct JoinLinksApplication {
    pool: PgPool,
    welcome_url: String,
}

impl JoinLinksApplication {
    pub fn new(pool: PgPool, public_url: &str) -> Self {
        let welcome_url = public_url
            .trim_end_matches("/auth/complete")
            .trim_end_matches('/')
            .to_owned()
            + "/welcome";
        Self { pool, welcome_url }
    }

    async fn create_link(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        body: &CreateJoinLinkRequest,
    ) -> Result<JoinLinkResponse, JoinError> {
        if !matches!(body.role.as_str(), "viewer" | "editor") {
            return Err(JoinError::Invalid);
        }
        let expires_at = body
            .expires_at
            .as_deref()
            .map(|value| {
                DateTime::parse_from_rfc3339(value)
                    .map(|date| date.with_timezone(&Utc))
                    .map_err(|_| JoinError::Invalid)
            })
            .transpose()?;
        if expires_at.is_some_and(|expires| expires <= Utc::now()) {
            return Err(JoinError::Invalid);
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        lock_active_space_owner(&mut transaction, space_id, actor_id).await?;
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, JoinLinkRow>(
            "INSERT INTO space_join_links (id, space_id, created_by, role, expires_at) \
             SELECT $1, s.id, $3, $4, $5 FROM spaces s WHERE s.id = $2 \
             RETURNING id, space_id, (SELECT name FROM spaces WHERE id = space_id) AS space_name, \
                       role, created_at, expires_at, revoked_at",
        )
        .bind(id)
        .bind(space_id)
        .bind(actor_id)
        .bind(&body.role)
        .bind(expires_at)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?
        .ok_or(JoinError::NotFound)?;
        transaction
            .commit()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        Ok(self.map_link(row))
    }

    async fn list_links(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Vec<JoinLinkResponse>, JoinError> {
        require_owner(&self.pool, space_id, actor_id).await?;
        let rows = sqlx::query_as::<_, JoinLinkRow>(
            "SELECT l.id, l.space_id, s.name AS space_name, l.role, l.created_at, \
                    l.expires_at, l.revoked_at \
             FROM space_join_links l JOIN spaces s ON s.id = l.space_id \
             WHERE l.space_id = $1 ORDER BY l.created_at DESC LIMIT 100",
        )
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        Ok(rows.into_iter().map(|row| self.map_link(row)).collect())
    }

    async fn revoke_link(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        link_id: Uuid,
    ) -> Result<(), JoinError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        lock_active_space_owner(&mut transaction, space_id, actor_id).await?;
        let result = sqlx::query(
            "UPDATE space_join_links SET revoked_at = now() \
             WHERE id = $1 AND space_id = $2 AND revoked_at IS NULL",
        )
        .bind(link_id)
        .bind(space_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        if result.rows_affected() == 1 {
            transaction
                .commit()
                .await
                .map_err(|_| JoinError::Unavailable)
        } else {
            Err(JoinError::NotFound)
        }
    }

    async fn request_access(
        &self,
        link_id: Uuid,
        requester_id: Uuid,
    ) -> Result<JoinRequestResponse, JoinError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        let link = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT l.space_id, s.name, l.role FROM space_join_links l \
             JOIN spaces s ON s.id = l.space_id \
             WHERE l.id = $1 AND l.revoked_at IS NULL \
               AND (l.expires_at IS NULL OR l.expires_at > now()) FOR SHARE OF l",
        )
        .bind(link_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?
        .ok_or(JoinError::NotFound)?;
        let is_member = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM space_memberships WHERE space_id = $1 AND user_id = $2)",
        )
        .bind(link.0)
        .bind(requester_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        if is_member {
            return Err(JoinError::AlreadyMember);
        }
        if let Some(row) = sqlx::query_as::<_, JoinRequestRow>(
            "SELECT r.id, r.link_id, r.space_id, s.name AS space_name, r.requester_id, \
                    e.email_normalized AS requester_email, r.requested_role, r.status, \
                    r.requested_at, r.decided_at \
             FROM space_join_requests r JOIN spaces s ON s.id = r.space_id \
             JOIN email_identities e ON e.user_id = r.requester_id \
             WHERE r.space_id = $1 AND r.requester_id = $2 AND r.status = 'pending'",
        )
        .bind(link.0)
        .bind(requester_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?
        {
            transaction
                .commit()
                .await
                .map_err(|_| JoinError::Unavailable)?;
            return Ok(row.into());
        }
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, JoinRequestRow>(
            "INSERT INTO space_join_requests \
             (id, link_id, space_id, requester_id, requested_role) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, link_id, space_id, ''::text AS space_name, requester_id, \
                       ''::text AS requester_email, requested_role, status, requested_at, decided_at",
        )
        .bind(id)
        .bind(link_id)
        .bind(link.0)
        .bind(requester_id)
        .bind(&link.2)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            if error.as_database_error().is_some_and(|db| db.code().as_deref() == Some("23505")) {
                JoinError::RequestPending
            } else {
                JoinError::Unavailable
            }
        })?;
        let requester_email = sqlx::query_scalar::<_, String>(
            "SELECT email_normalized FROM email_identities WHERE user_id = $1",
        )
        .bind(requester_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        crate::notifications::enqueue_space_owners(
            &mut transaction,
            link.0,
            requester_id,
            &crate::notifications::NotificationContent {
                event_type: "join_request",
                title: "A user asked to join your Space",
                body: &format!("{requester_email} requested {} access.", link.2),
                destination: "members",
            },
        )
        .await
        .map_err(|_| JoinError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        Ok(JoinRequestResponse {
            space_name: link.1,
            requester_email: None,
            ..row.into()
        })
    }

    async fn list_my_requests(
        &self,
        requester_id: Uuid,
    ) -> Result<Vec<JoinRequestResponse>, JoinError> {
        let rows = sqlx::query_as::<_, JoinRequestRow>(
            "SELECT r.id, r.link_id, r.space_id, s.name AS space_name, r.requester_id, \
                    e.email_normalized AS requester_email, r.requested_role, r.status, \
                    r.requested_at, r.decided_at \
             FROM space_join_requests r JOIN spaces s ON s.id = r.space_id \
             JOIN email_identities e ON e.user_id = r.requester_id \
             WHERE r.requester_id = $1 ORDER BY r.requested_at DESC LIMIT 100",
        )
        .bind(requester_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let mut response: JoinRequestResponse = row.into();
                response.requester_email = None;
                response
            })
            .collect())
    }

    async fn list_pending_requests(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Vec<JoinRequestResponse>, JoinError> {
        require_owner(&self.pool, space_id, actor_id).await?;
        let rows = sqlx::query_as::<_, JoinRequestRow>(
            "SELECT r.id, r.link_id, r.space_id, s.name AS space_name, r.requester_id, \
                    e.email_normalized AS requester_email, r.requested_role, r.status, \
                    r.requested_at, r.decided_at \
             FROM space_join_requests r JOIN spaces s ON s.id = r.space_id \
             JOIN email_identities e ON e.user_id = r.requester_id \
             WHERE r.space_id = $1 AND r.status = 'pending' \
             ORDER BY r.requested_at ASC LIMIT 100",
        )
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn decide_request(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        request_id: Uuid,
        decision: &str,
    ) -> Result<JoinRequestResponse, JoinError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        lock_active_space_owner(&mut transaction, space_id, actor_id).await?;
        if !matches!(decision, "approved" | "declined") {
            return Err(JoinError::Invalid);
        }
        let row = sqlx::query_as::<_, JoinRequestRow>(
            "SELECT r.id, r.link_id, r.space_id, s.name AS space_name, r.requester_id, \
                    e.email_normalized AS requester_email, r.requested_role, r.status, \
                    r.requested_at, r.decided_at \
             FROM space_join_requests r JOIN spaces s ON s.id = r.space_id \
             JOIN email_identities e ON e.user_id = r.requester_id \
             WHERE r.id = $1 AND r.space_id = $2 FOR UPDATE OF r",
        )
        .bind(request_id)
        .bind(space_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?
        .ok_or(JoinError::NotFound)?;
        if row.status != "pending" {
            return Err(JoinError::AlreadyDecided);
        }
        if decision == "approved" {
            let inserted = sqlx::query(
                "INSERT INTO space_memberships (space_id, user_id, role) \
                 SELECT $1, $2, $3 WHERE NOT EXISTS ( \
                   SELECT 1 FROM space_memberships WHERE space_id = $1 AND user_id = $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(space_id)
            .bind(row.requester_id)
            .bind(&row.requested_role)
            .execute(&mut *transaction)
            .await
            .map_err(|_| JoinError::Unavailable)?;
            if inserted.rows_affected() == 0 {
                return Err(JoinError::AlreadyMember);
            }
        }
        let updated = sqlx::query_as::<_, JoinRequestRow>(
            "UPDATE space_join_requests r SET status = $3, decided_at = now(), decided_by = $4 \
             FROM spaces s, email_identities e \
             WHERE r.id = $1 AND r.space_id = $2 AND s.id = r.space_id \
               AND e.user_id = r.requester_id \
             RETURNING r.id, r.link_id, r.space_id, s.name AS space_name, r.requester_id, \
                       e.email_normalized AS requester_email, r.requested_role, r.status, \
                       r.requested_at, r.decided_at",
        )
        .bind(request_id)
        .bind(space_id)
        .bind(decision)
        .bind(actor_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| JoinError::Unavailable)?;
        crate::notifications::enqueue_notification(
            &mut transaction,
            space_id,
            actor_id,
            updated.requester_id,
            &crate::notifications::NotificationContent {
                event_type: "join_request_result",
                title: if decision == "approved" {
                    "Your join request was approved"
                } else {
                    "Your join request was declined"
                },
                body: &format!(
                    "Your request to join {} was {}.",
                    updated.space_name, decision
                ),
                destination: "members",
            },
        )
        .await
        .map_err(|_| JoinError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| JoinError::Unavailable)?;
        Ok(updated.into())
    }

    fn map_link(&self, row: JoinLinkRow) -> JoinLinkResponse {
        JoinLinkResponse {
            id: row.id,
            space_id: row.space_id,
            space_name: row.space_name,
            role: row.role,
            created_at: row.created_at.to_rfc3339(),
            expires_at: row.expires_at.map(|value| value.to_rfc3339()),
            revoked_at: row.revoked_at.map(|value| value.to_rfc3339()),
            url: format!("{}?join={}", self.welcome_url, row.id),
        }
    }
}

async fn require_owner(pool: &PgPool, space_id: Uuid, user_id: Uuid) -> Result<(), JoinError> {
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
    )
    .bind(space_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| JoinError::Unavailable)?;
    match role.as_deref() {
        Some("owner") => Ok(()),
        Some(_) => Err(JoinError::NotOwner),
        None => Err(JoinError::NotFound),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateJoinLinkRequest {
    pub role: String,
    /// RFC 3339 UTC timestamp, or null for a link that does not expire.
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct JoinLinkResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    pub role: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct JoinRequestResponse {
    pub id: Uuid,
    pub link_id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    pub requester_id: Uuid,
    pub requester_email: Option<String>,
    pub role: String,
    pub status: String,
    pub requested_at: String,
    pub decided_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct JoinRequestDecision {
    pub decision: String,
}

#[derive(sqlx::FromRow)]
struct JoinLinkRow {
    id: Uuid,
    space_id: Uuid,
    space_name: String,
    role: String,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct JoinRequestRow {
    id: Uuid,
    link_id: Uuid,
    space_id: Uuid,
    space_name: String,
    requester_id: Uuid,
    requester_email: String,
    requested_role: String,
    status: String,
    requested_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
}

impl From<JoinRequestRow> for JoinRequestResponse {
    fn from(row: JoinRequestRow) -> Self {
        Self {
            id: row.id,
            link_id: row.link_id,
            space_id: row.space_id,
            space_name: row.space_name,
            requester_id: row.requester_id,
            requester_email: Some(row.requester_email),
            role: row.requested_role,
            status: row.status,
            requested_at: row.requested_at.to_rfc3339(),
            decided_at: row.decided_at.map(|value| value.to_rfc3339()),
        }
    }
}

#[derive(Debug)]
enum JoinError {
    Invalid,
    Unauthorized,
    NotOwner,
    NotFound,
    AlreadyMember,
    RequestPending,
    AlreadyDecided,
    Unavailable,
}

impl From<OwnerAuthorizationError> for JoinError {
    fn from(error: OwnerAuthorizationError) -> Self {
        match error {
            OwnerAuthorizationError::NotFound => Self::NotFound,
            OwnerAuthorizationError::NotOwner => Self::NotOwner,
            OwnerAuthorizationError::Unavailable => Self::Unavailable,
        }
    }
}

async fn auth_account(
    auth: &AuthApplication,
    request: &HttpRequest,
) -> Result<crate::auth::Account, JoinError> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or(JoinError::Unauthorized)?;
    auth.current_account(token)
        .await
        .map_err(|error| match error {
            AuthError::Unauthorized => JoinError::Unauthorized,
            _ => JoinError::Unavailable,
        })
}

fn join_error(error: JoinError) -> HttpResponse {
    let (status, title, code) = match error {
        JoinError::Invalid => (
            StatusCode::BAD_REQUEST,
            "Invalid join-link request",
            "invalid_join_request",
        ),
        JoinError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "Authentication failed",
            "authentication_failed",
        ),
        JoinError::NotOwner => (
            StatusCode::FORBIDDEN,
            "Only the Space Owner can manage join access",
            "owner_required",
        ),
        JoinError::NotFound => (
            StatusCode::NOT_FOUND,
            "Join resource not found",
            "join_resource_not_found",
        ),
        JoinError::AlreadyMember => (
            StatusCode::CONFLICT,
            "You already belong to this Space",
            "already_a_member",
        ),
        JoinError::RequestPending => (
            StatusCode::CONFLICT,
            "A join request is already pending",
            "join_request_pending",
        ),
        JoinError::AlreadyDecided => (
            StatusCode::CONFLICT,
            "This join request has already been decided",
            "join_request_decided",
        ),
        JoinError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    };
    HttpResponse::build(status)
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title,
            status: status.as_u16(),
            code,
        })
}

#[utoipa::path(post, path = "/spaces/{space_id}/join-links", params(("space_id" = Uuid, Path)), request_body = CreateJoinLinkRequest, security(("accessBearer" = [])), responses((status = 201, body = JoinLinkResponse), (status = 400, body = Problem), (status = 403, body = Problem))) ]
#[post("/spaces/{space_id}/join-links")]
pub async fn create_join_link(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
    body: web::Json<CreateJoinLinkRequest>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    match app.create_link(path.into_inner(), account.id, &body).await {
        Ok(result) => HttpResponse::Created().json(result),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(get, path = "/spaces/{space_id}/join-links", params(("space_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 200, body = [JoinLinkResponse]), (status = 403, body = Problem))) ]
#[get("/spaces/{space_id}/join-links")]
pub async fn list_join_links(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    match app.list_links(path.into_inner(), account.id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(delete, path = "/spaces/{space_id}/join-links/{link_id}", params(("space_id" = Uuid, Path), ("link_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem))) ]
#[delete("/spaces/{space_id}/join-links/{link_id}")]
pub async fn revoke_join_link(
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    let (space_id, link_id) = path.into_inner();
    match app.revoke_link(space_id, account.id, link_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(post, path = "/join-links/{link_id}/requests", params(("link_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 201, body = JoinRequestResponse), (status = 401, body = Problem), (status = 404, body = Problem), (status = 409, body = Problem))) ]
#[post("/join-links/{link_id}/requests")]
pub async fn request_join_access(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    match app.request_access(path.into_inner(), account.id).await {
        Ok(result) => HttpResponse::Created().json(result),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(get, path = "/join-requests/current", security(("accessBearer" = [])), responses((status = 200, body = [JoinRequestResponse]), (status = 401, body = Problem))) ]
#[get("/join-requests/current")]
pub async fn list_my_join_requests(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    match app.list_my_requests(account.id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(get, path = "/spaces/{space_id}/join-requests", params(("space_id" = Uuid, Path)), security(("accessBearer" = [])), responses((status = 200, body = [JoinRequestResponse]), (status = 403, body = Problem))) ]
#[get("/spaces/{space_id}/join-requests")]
pub async fn list_pending_join_requests(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    match app
        .list_pending_requests(path.into_inner(), account.id)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => join_error(error),
    }
}

#[utoipa::path(post, path = "/spaces/{space_id}/join-requests/{request_id}/decision", params(("space_id" = Uuid, Path), ("request_id" = Uuid, Path)), request_body = JoinRequestDecision, security(("accessBearer" = [])), responses((status = 200, body = JoinRequestResponse), (status = 403, body = Problem), (status = 404, body = Problem), (status = 409, body = Problem))) ]
#[post("/spaces/{space_id}/join-requests/{request_id}/decision")]
pub async fn decide_join_request(
    request: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    auth: web::Data<AuthApplication>,
    app: web::Data<JoinLinksApplication>,
    body: web::Json<JoinRequestDecision>,
) -> HttpResponse {
    let account = match auth_account(&auth, &request).await {
        Ok(account) => account,
        Err(error) => return join_error(error),
    };
    let (space_id, request_id) = path.into_inner();
    match app
        .decide_request(space_id, account.id, request_id, &body.decision)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => join_error(error),
    }
}
