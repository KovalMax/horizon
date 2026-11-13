use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(super) const ACCESS_TOKEN_TTL_SECONDS: i64 = 10 * 60;
pub(super) const MAGIC_LINK_TTL_MINUTES: i64 = 30;
pub(super) const REFRESH_IDLE_DAYS: i64 = 30;
pub(super) const SESSION_ABSOLUTE_DAYS: i64 = 90;
pub(super) const MAGIC_LINK_EMAIL_HOURLY_LIMIT: i64 = 5;
pub(super) const MAGIC_LINK_IP_HOURLY_LIMIT: i64 = 20;
pub(super) const MAGIC_LINK_RESEND_COOLDOWN_SECONDS: i64 = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: Uuid,
    pub email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSet {
    pub access_token: String,
    pub access_expires_in: i64,
    pub refresh_token: String,
    pub refresh_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub account: Account,
    pub tokens: TokenSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPrincipal {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthError {
    InvalidRequest,
    RateLimited,
    Unauthorized,
    Unavailable,
    EmailDelivery,
    Internal,
}
