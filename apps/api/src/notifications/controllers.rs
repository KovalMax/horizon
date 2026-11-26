use actix_web::{
    delete, get,
    http::{header, StatusCode},
    post, put, web, HttpRequest, HttpResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{AuthApplication, AuthError, Problem},
    pagination::PageWindow,
};

use super::{
    cursor::NotificationCursor,
    push::{
        self, PushConfig, PushPreferences, PutPushSubscription, RegistrationError,
        RemovePushSubscription,
    },
    requests::{UpdateNotificationPreferencesRequest, UpdateNotificationReadRequest},
    services::{NotificationError, NotificationsApplication},
};

#[utoipa::path(
    get,
    path = "/notifications/push-config",
    responses((status = 200, body = PushConfig))
)]
#[get("/notifications/push-config")]
pub async fn push_config() -> HttpResponse {
    HttpResponse::Ok().json(PushConfig {
        public_key: super::PushWorker::public_key(),
    })
}

#[utoipa::path(
    post,
    path = "/notifications/push-subscription/status",
    request_body = RemovePushSubscription,
    security(("accessBearer" = [])),
    responses((status = 200, body = PushPreferences), (status = 204), (status = 400, body = Problem), (status = 401, body = Problem))
)]
#[post("/notifications/push-subscription/status")]
pub async fn push_subscription_status(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    pool: web::Data<sqlx::PgPool>,
    subscription: web::Json<RemovePushSubscription>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    match push::preferences(&pool, user_id, &subscription.endpoint).await {
        Ok(Some(preferences)) => HttpResponse::Ok().json(preferences),
        Ok(None) => HttpResponse::NoContent().finish(),
        Err(error) => push_result(Err(error)),
    }
}

fn push_result(result: Result<(), RegistrationError>) -> HttpResponse {
    match result {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(RegistrationError::Invalid) => problem(
            StatusCode::BAD_REQUEST,
            "Push subscription is invalid or unsupported",
            "invalid_push_subscription",
        ),
        Err(RegistrationError::Conflict) => problem(
            StatusCode::CONFLICT,
            "Create a new push subscription for this account",
            "push_subscription_conflict",
        ),
        Err(RegistrationError::Unavailable) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    put,
    path = "/notifications/push-subscription",
    request_body = PutPushSubscription,
    security(("accessBearer" = [])),
    responses((status = 204), (status = 400, body = Problem), (status = 401, body = Problem), (status = 409, body = Problem))
)]
#[put("/notifications/push-subscription")]
pub async fn register_push_subscription(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    pool: web::Data<sqlx::PgPool>,
    subscription: web::Json<PutPushSubscription>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    let Some(principal) =
        bearer_token(&request).and_then(|token| auth.access_principal(token).ok())
    else {
        return authentication_error(StatusCode::UNAUTHORIZED);
    };
    push_result(push::register(&pool, user_id, principal.session_id, &subscription).await)
}

#[utoipa::path(
    delete,
    path = "/notifications/push-subscription",
    request_body = RemovePushSubscription,
    security(("accessBearer" = [])),
    responses((status = 204), (status = 400, body = Problem), (status = 401, body = Problem))
)]
#[delete("/notifications/push-subscription")]
pub async fn remove_push_subscription(
    request: HttpRequest,
    auth: web::Data<AuthApplication>,
    pool: web::Data<sqlx::PgPool>,
    subscription: web::Json<RemovePushSubscription>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(status) => return authentication_error(status),
    };
    push_result(push::remove(&pool, user_id, &subscription.endpoint).await)
}

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

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationPageQuery {
    /// Maximum items to return (defaults to 50, maximum 100).
    #[param(minimum = 1, maximum = 100)]
    limit: Option<i64>,
    /// Deprecated offset pagination. Cannot be combined with a cursor.
    #[param(minimum = 0, maximum = 1000000)]
    offset: Option<i64>,
    /// Opaque continuation returned in `next_cursor`.
    cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/notifications",
    params(NotificationPageQuery),
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::NotificationPageResponse), (status = 400, body = Problem), (status = 401, body = Problem))
)]
#[get("/notifications")]
pub async fn list_notifications(
    request: HttpRequest,
    query: web::Query<NotificationPageQuery>,
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
    let cursor = match query.cursor.as_deref() {
        None => None,
        Some(cursor) => match NotificationCursor::decode(cursor) {
            Some(cursor) if query.offset.is_none() => Some(cursor),
            _ => {
                return problem(
                    StatusCode::BAD_REQUEST,
                    "Invalid page cursor",
                    "invalid_page",
                )
            }
        },
    };
    match notifications.list_for_user(user_id, page, cursor).await {
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
