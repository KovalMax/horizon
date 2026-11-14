use actix_web::{
    delete, get,
    http::{header, StatusCode},
    patch, post, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};

use super::{
    requests::{CreateSpaceRequest, TransferOwnershipRequest},
    responses::{DeletedSpaceResponse, TransferOwnershipResponse},
    services::{CreateSpaceError, SpaceLifecycleError, SpacesApplication},
};

fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

async fn authenticated_user(
    auth: &AuthApplication,
    request: &HttpRequest,
) -> Result<Uuid, AuthenticationFailure> {
    let Some(token) = bearer_token(request) else {
        return Err(AuthenticationFailure::Unauthorized);
    };
    auth.current_account(token)
        .await
        .map(|account| account.id)
        .map_err(|error| match error {
            AuthError::Unauthorized => AuthenticationFailure::Unauthorized,
            _ => AuthenticationFailure::Unavailable,
        })
}

enum AuthenticationFailure {
    Unauthorized,
    Unavailable,
}

impl AuthenticationFailure {
    fn into_response(self) -> HttpResponse {
        match self {
            Self::Unauthorized => unauthorized(),
            Self::Unavailable => unavailable(),
        }
    }
}

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized()
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title: "Authentication failed",
            status: StatusCode::UNAUTHORIZED.as_u16(),
            code: "authentication_failed",
        })
}

fn unavailable() -> HttpResponse {
    HttpResponse::ServiceUnavailable()
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title: "Service unavailable",
            status: StatusCode::SERVICE_UNAVAILABLE.as_u16(),
            code: "database_unavailable",
        })
}

fn invalid_name() -> HttpResponse {
    HttpResponse::BadRequest()
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title: "Invalid Space name",
            status: StatusCode::BAD_REQUEST.as_u16(),
            code: "invalid_space_name",
        })
}

fn lifecycle_error(error: SpaceLifecycleError) -> HttpResponse {
    let (status, title, code) = match error {
        SpaceLifecycleError::OwnerRequired => (
            StatusCode::FORBIDDEN,
            "Only the Space Owner can manage its lifecycle",
            "owner_required",
        ),
        SpaceLifecycleError::TargetNotMember => (
            StatusCode::BAD_REQUEST,
            "The new Owner must already be a Member of this Space",
            "target_not_member",
        ),
        SpaceLifecycleError::InvalidTransfer => (
            StatusCode::BAD_REQUEST,
            "Choose a different Member as the new Owner",
            "invalid_ownership_transfer",
        ),
        SpaceLifecycleError::PersonalTransferUnsupported => (
            StatusCode::CONFLICT,
            "Personal Spaces cannot transfer ownership",
            "personal_space_transfer_unsupported",
        ),
        SpaceLifecycleError::NotFound => (
            StatusCode::NOT_FOUND,
            "Space not found or no longer available",
            "space_not_found",
        ),
        SpaceLifecycleError::Unavailable => (
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

#[utoipa::path(
    get,
    path = "/spaces",
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "Spaces the authenticated member can access", body = [super::responses::SpaceResponse]),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[get("/spaces")]
pub async fn list_spaces(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces.list_for_member(user_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/spaces",
    request_body = CreateSpaceRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 201, description = "Shared Space created with the caller as Owner", body = super::responses::SpaceResponse),
        (status = 400, description = "Space name must contain 1 to 80 characters", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[post("/spaces")]
pub async fn create_shared_space(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
    body: web::Json<CreateSpaceRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces.create_shared(&body.name, user_id).await {
        Ok(space) => HttpResponse::Created().json(space),
        Err(CreateSpaceError::InvalidName) => invalid_name(),
        Err(CreateSpaceError::Unavailable(_)) => unavailable(),
    }
}

#[utoipa::path(
    patch,
    path = "/spaces/{space_id}/owner",
    params(("space_id" = Uuid, Path)),
    request_body = TransferOwnershipRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 200, body = TransferOwnershipResponse),
        (status = 400, body = Problem),
        (status = 401, body = Problem),
        (status = 403, body = Problem),
        (status = 404, body = Problem),
        (status = 409, body = Problem)
    )
)]
#[patch("/spaces/{space_id}/owner")]
pub async fn transfer_space_ownership(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
    body: web::Json<TransferOwnershipRequest>,
) -> HttpResponse {
    let owner_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces
        .transfer_ownership(path.into_inner(), owner_id, body.new_owner_id)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => lifecycle_error(error),
    }
}

#[utoipa::path(
    delete,
    path = "/spaces/{space_id}",
    params(("space_id" = Uuid, Path)),
    security(("accessBearer" = [])),
    responses((status = 200, body = DeletedSpaceResponse), (status = 401, body = Problem), (status = 404, body = Problem))
)]
#[delete("/spaces/{space_id}")]
pub async fn delete_space(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
) -> HttpResponse {
    let owner_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces.delete_space(path.into_inner(), owner_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => lifecycle_error(error),
    }
}

#[utoipa::path(
    get,
    path = "/spaces/deleted",
    security(("accessBearer" = [])),
    responses((status = 200, body = [DeletedSpaceResponse]), (status = 401, body = Problem))
)]
#[get("/spaces/deleted")]
pub async fn list_deleted_spaces(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
) -> HttpResponse {
    let owner_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces.list_deleted_for_owner(owner_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/spaces/{space_id}/restore",
    params(("space_id" = Uuid, Path)),
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::SpaceResponse), (status = 401, body = Problem), (status = 404, body = Problem))
)]
#[post("/spaces/{space_id}/restore")]
pub async fn restore_space(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    spaces: web::Data<SpacesApplication>,
) -> HttpResponse {
    let owner_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match spaces.restore_space(path.into_inner(), owner_id).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => lifecycle_error(error),
    }
}
