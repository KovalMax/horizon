use actix_web::http::Uri;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct PutPushSubscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub calendar_reminders: bool,
    pub task_reminders: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct RemovePushSubscription {
    pub endpoint: String,
}

#[derive(Serialize, ToSchema)]
pub struct PushConfig {
    pub public_key: Option<String>,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct PushPreferences {
    pub calendar_reminders: bool,
    pub task_reminders: bool,
}

pub(super) enum RegistrationError {
    Invalid,
    Conflict,
    Unavailable,
}

// Keep validation narrow before adding any worker that makes outbound requests.
pub(super) fn supported_endpoint(endpoint: &str) -> bool {
    if endpoint.len() > 2048 {
        return false;
    }
    let Ok(uri) = endpoint.parse::<Uri>() else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && matches!(
            uri.authority().map(|authority| authority.as_str()),
            Some("fcm.googleapis.com" | "updates.push.services.mozilla.com" | "web.push.apple.com")
        )
        && uri.path().len() > 1
        && !endpoint.contains('#')
}

pub(super) async fn register(
    pool: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    subscription: &PutPushSubscription,
) -> Result<(), RegistrationError> {
    let p256dh = URL_SAFE_NO_PAD
        .decode(subscription.p256dh.as_bytes())
        .map_err(|_| RegistrationError::Invalid)?;
    let auth = URL_SAFE_NO_PAD
        .decode(subscription.auth.as_bytes())
        .map_err(|_| RegistrationError::Invalid)?;
    if !supported_endpoint(&subscription.endpoint)
        || p256dh.len() != 65
        || p256dh.first() != Some(&4)
        || auth.len() != 16
    {
        return Err(RegistrationError::Invalid);
    }
    let result = sqlx::query(
        "INSERT INTO web_push_subscriptions \
         (endpoint, user_id, p256dh, auth_secret, calendar_reminders, task_reminders, session_id) \
         SELECT $1, $2, $3, $4, $5, $6, $7 FROM auth_sessions \
         WHERE id = $7 AND user_id = $2 AND revoked_at IS NULL AND absolute_expires_at > now() \
         ON CONFLICT (endpoint) DO UPDATE SET \
         p256dh = EXCLUDED.p256dh, auth_secret = EXCLUDED.auth_secret, \
         calendar_reminders = EXCLUDED.calendar_reminders, task_reminders = EXCLUDED.task_reminders, \
         session_id = EXCLUDED.session_id, updated_at = now() \
         WHERE web_push_subscriptions.user_id = EXCLUDED.user_id",
    )
    .bind(&subscription.endpoint)
    .bind(user_id)
    .bind(p256dh)
    .bind(auth)
    .bind(subscription.calendar_reminders)
    .bind(subscription.task_reminders)
    .bind(session_id)
    .execute(pool)
    .await
    .map_err(|_| RegistrationError::Unavailable)?;
    if result.rows_affected() == 0 {
        return Err(RegistrationError::Conflict);
    }
    Ok(())
}

pub(super) async fn remove(
    pool: &PgPool,
    user_id: Uuid,
    endpoint: &str,
) -> Result<(), RegistrationError> {
    if !supported_endpoint(endpoint) {
        return Err(RegistrationError::Invalid);
    }
    sqlx::query("DELETE FROM web_push_subscriptions WHERE user_id = $1 AND endpoint = $2")
        .bind(user_id)
        .bind(endpoint)
        .execute(pool)
        .await
        .map_err(|_| RegistrationError::Unavailable)?;
    Ok(())
}

pub(super) async fn preferences(
    pool: &PgPool,
    user_id: Uuid,
    endpoint: &str,
) -> Result<Option<PushPreferences>, RegistrationError> {
    if !supported_endpoint(endpoint) {
        return Err(RegistrationError::Invalid);
    }
    sqlx::query_as(
        "SELECT calendar_reminders, task_reminders FROM web_push_subscriptions \
         WHERE user_id = $1 AND endpoint = $2",
    )
    .bind(user_id)
    .bind(endpoint)
    .fetch_optional(pool)
    .await
    .map_err(|_| RegistrationError::Unavailable)
}
