use actix_web::{
    delete, get,
    http::{header, StatusCode},
    post, put, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};
use crate::pagination::{PageQuery, PageWindow};

use super::{models::EventError, requests::CreateEventRequest, services::EventsApplication};

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

fn event_error(error: EventError) -> HttpResponse {
    match error {
        EventError::Invalid => problem(
            StatusCode::BAD_REQUEST,
            "Invalid Calendar event",
            "invalid_calendar_event",
        ),
        EventError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "Insufficient Space role",
            "insufficient_space_role",
        ),
        EventError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "Calendar event not found",
            "calendar_event_not_found",
        ),
        EventError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    get,
    path = "/events",
    params(PageQuery),
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "A page of Calendar events in Spaces the authenticated Member can access", body = super::responses::EventPageResponse),
        (status = 400, description = "Page offset is invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[get("/events")]
pub async fn list_events(
    request: HttpRequest,
    query: web::Query<PageQuery>,
    auth: web::Data<AuthApplication>,
    events: web::Data<EventsApplication>,
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
    match events.list_for_member(user_id, page).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => event_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/spaces/{space_id}/events",
    params(("space_id" = Uuid, Path, description = "Space containing the event")),
    request_body = CreateEventRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 201, description = "Calendar event created", body = super::responses::EventResponse),
        (status = 400, description = "Event fields are invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot create events", body = Problem),
        (status = 404, description = "Space not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[post("/spaces/{space_id}/events")]
pub async fn create_event(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    events: web::Data<EventsApplication>,
    body: web::Json<CreateEventRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match events.create(path.into_inner(), user_id, &body).await {
        Ok(event) => HttpResponse::Created().json(event),
        Err(error) => event_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/events/{event_id}",
    params(("event_id" = Uuid, Path, description = "Calendar event to update")),
    request_body = CreateEventRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "Calendar event updated", body = super::responses::EventResponse),
        (status = 400, description = "Event fields are invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot update events", body = Problem),
        (status = 404, description = "Event not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[put("/events/{event_id}")]
pub async fn update_event(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    events: web::Data<EventsApplication>,
    body: web::Json<CreateEventRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match events.update(path.into_inner(), user_id, &body).await {
        Ok(event) => HttpResponse::Ok().json(event),
        Err(error) => event_error(error),
    }
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}",
    params(("event_id" = Uuid, Path, description = "Calendar event to delete")),
    security(("accessBearer" = [])),
    responses(
        (status = 204, description = "Calendar event deleted"),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot delete events", body = Problem),
        (status = 404, description = "Event not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[delete("/events/{event_id}")]
pub async fn delete_event(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    events: web::Data<EventsApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match events.delete(path.into_inner(), user_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => event_error(error),
    }
}
