use actix_web::{
    get,
    http::{header, StatusCode},
    post, put, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::{
    auth::{AuthApplication, AuthError, Problem},
    pagination::{PageQuery, PageWindow},
};

use super::{
    requests::{UpdateNotificationPreferencesRequest, UpdateNotificationReadRequest},
    services::{NotificationError, NotificationsApplication},
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
) -> Result<Uuid, StatusCode> {
    let Some(token) = bearer_token(request) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    auth.current_account(token)
        .await
        .map(|account| account.id)
        .map_err(|error| match error {
            AuthError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })
}

fn authentication_error(status: StatusCode) -> HttpResponse {
    if status == StatusCode::UNAUTHORIZED {
        problem(status, "Authentication failed", "authentication_failed")
    } else {
        problem(status, "Service unavailable", "database_unavailable")
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

fn notification_error(error: NotificationError) -> HttpResponse {
    match error {
        NotificationError::InvalidPreferences => problem(
            StatusCode::BAD_REQUEST,
            "Notification preferences are invalid",
            "invalid_notification_preferences",
        ),
        NotificationError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "Notification not found",
            "notification_not_found",
        ),
        NotificationError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    get,
    path = "/notifications",
    params(PageQuery),
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::NotificationPageResponse), (status = 400, body = Problem), (status = 401, body = Problem))
)]
#[get("/notifications")]
pub async fn list_notifications(
    request: HttpRequest,
    query: web::Query<PageQuery>,
    auth: web::Data<AuthApplication>,
    notifications: web::Data<NotificationsApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    let Some(page) = PageWindow::new(query.offset, query.limit) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid page offset or limit",
            "invalid_page",
        );
    };
    match notifications.list_for_user(user_id, page).await {
        Ok(page) => HttpResponse::Ok().json(page),
        Err(error) => notification_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/notifications/{notification_id}/read",
    params(("notification_id" = Uuid, Path)),
    request_body = UpdateNotificationReadRequest,
    security(("accessBearer" = [])),
    responses((status = 204), (status = 401, body = Problem), (status = 404, body = Problem))
)]
#[put("/notifications/{notification_id}/read")]
pub async fn mark_notification_read(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    notifications: web::Data<NotificationsApplication>,
    body: web::Json<UpdateNotificationReadRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    match notifications
        .set_read(user_id, path.into_inner(), body.read)
        .await
    {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => notification_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/notifications/read-all",
    security(("accessBearer" = [])),
    responses((status = 204), (status = 401, body = Problem))
)]
#[post("/notifications/read-all")]
pub async fn mark_all_notifications_read(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    notifications: web::Data<NotificationsApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    match notifications.mark_all_read(user_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(error) => notification_error(error),
    }
}

#[utoipa::path(
    get,
    path = "/notifications/preferences",
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::NotificationSettingsResponse), (status = 401, body = Problem))
)]
#[get("/notifications/preferences")]
pub async fn notification_preferences(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    notifications: web::Data<NotificationsApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    match notifications.settings(user_id).await {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(error) => notification_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/notifications/preferences",
    request_body = UpdateNotificationPreferencesRequest,
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::NotificationSettingsResponse), (status = 400, body = Problem), (status = 401, body = Problem))
)]
#[put("/notifications/preferences")]
pub async fn update_notification_preferences(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    notifications: web::Data<NotificationsApplication>,
    body: web::Json<UpdateNotificationPreferencesRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    match notifications
        .update_settings(
            user_id,
            &body.preferences,
            &body.muted_space_ids,
            &body.default_reminder_time,
            &body.time_zone,
        )
        .await
    {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(error) => notification_error(error),
    }
}
