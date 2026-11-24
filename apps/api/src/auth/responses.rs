use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::AuthenticatedSession;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountResponse {
    pub id: Uuid,
    pub email: String,
}

impl From<super::models::Account> for AccountResponse {
    fn from(account: super::models::Account) -> Self {
        Self {
            id: account.id,
            email: account.email,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenPairResponse {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub refresh_expires_in: i64,
}

impl From<super::models::TokenSet> for TokenPairResponse {
    fn from(tokens: super::models::TokenSet) -> Self {
        Self {
            token_type: "Bearer".to_owned(),
            access_token: tokens.access_token,
            expires_in: tokens.access_expires_in,
            refresh_token: tokens.refresh_token,
            refresh_expires_in: (tokens.refresh_expires_at - chrono::Utc::now()).num_seconds(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationResponse {
    pub account: AccountResponse,
    pub tokens: TokenPairResponse,
}

impl From<AuthenticatedSession> for AuthenticationResponse {
    fn from(session: AuthenticatedSession) -> Self {
        Self {
            account: session.account.into(),
            tokens: session.tokens.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MagicLinkAccepted {
    pub challenge_id: uuid::Uuid,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Problem {
    pub r#type: &'static str,
    pub title: &'static str,
    pub status: u16,
    pub code: &'static str,
}
