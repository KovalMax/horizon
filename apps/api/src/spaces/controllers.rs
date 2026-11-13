use actix_web::{
    get,
    http::{header, StatusCode},
    post, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};

use super::{
    requests::CreateSpaceRequest,
    services::{CreateSpaceError, SpacesApplication},
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
