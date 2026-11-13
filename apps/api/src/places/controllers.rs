use actix_web::{
    get,
    http::{header, StatusCode},
    post, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};
use crate::pagination::{PageQuery, PageWindow};

use super::{models::PlaceError, requests::CreatePlaceRequest, services::PlacesApplication};

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
            Self::Unauthorized => problem(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
                "authentication_failed",
            ),
            Self::Unavailable => problem(
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable",
                "database_unavailable",
            ),
        }
    }
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

fn place_error(error: PlaceError) -> HttpResponse {
    match error {
        PlaceError::Invalid => problem(
            StatusCode::BAD_REQUEST,
            "Invalid Saved place",
            "invalid_saved_place",
        ),
        PlaceError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "Insufficient Space role",
            "insufficient_space_role",
        ),
        PlaceError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "Saved place not found",
            "saved_place_not_found",
        ),
        PlaceError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    get,
    path = "/places",
    params(PageQuery),
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "A page of Saved places in Spaces the Member can access", body = super::responses::PlacePageResponse),
        (status = 400, description = "Page offset is invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[get("/places")]
pub async fn list_places(
    request: HttpRequest,
    query: web::Query<PageQuery>,
    auth: web::Data<AuthApplication>,
    places: web::Data<PlacesApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    let Some(page) = PageWindow::new(query.offset, query.limit) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid page offset",
            "invalid_page_offset",
        );
    };
    match places.list_for_member(user_id, page).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => place_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/spaces/{space_id}/places",
    params(("space_id" = Uuid, Path, description = "Space containing the Saved place")),
    request_body = CreatePlaceRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 201, description = "Saved place created", body = super::responses::PlaceResponse),
        (status = 400, description = "Saved place fields are invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot save places", body = Problem),
        (status = 404, description = "Space not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[post("/spaces/{space_id}/places")]
pub async fn create_place(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    places: web::Data<PlacesApplication>,
    body: web::Json<CreatePlaceRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match places.create(path.into_inner(), user_id, &body).await {
        Ok(place) => HttpResponse::Created().json(place),
        Err(error) => place_error(error),
    }
}
