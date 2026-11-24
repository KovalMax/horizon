use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use super::{
    email::EmailSender,
    models::{
        AuthError, ACCESS_TOKEN_TTL_SECONDS, MAGIC_LINK_TTL_MINUTES, REFRESH_IDLE_DAYS,
        SESSION_ABSOLUTE_DAYS,
    },
    repositories::AuthRepository,
};

pub struct AuthApplication {
    pub(super) mailer: Arc<dyn EmailSender>,
    pub(super) token_service: Arc<dyn super::token_service::TokenService>,
    pub(super) public_url: String,
    pub(super) repository: Arc<dyn AuthRepository>,
}

impl AuthApplication {
    pub fn email_sender(&self) -> Arc<dyn super::email::EmailSender> {
        self.mailer.clone()
    }

    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    pub async fn redeem_magic_link(
        &self,
        token: &str,
    ) -> Result<super::models::AuthenticatedSession, AuthError> {
        if !(32..=256).contains(&token.len()) {
            return Err(AuthError::Unauthorized);
        }
        let now = Utc::now();
        let session_id = Uuid::new_v4();
        let refresh_token = self.token_service.create_secret();
        let redeemed = self
            .repository
            .redeem_magic_link(
                self.token_service.hash_secret(token),
                session_id,
                self.token_service.hash_secret(&refresh_token),
                now,
                now + chrono::Duration::days(REFRESH_IDLE_DAYS),
                now + chrono::Duration::days(SESSION_ABSOLUTE_DAYS),
            )
            .await?;
        let access_token = self
            .token_service
            .issue_access_token(redeemed.account.id, redeemed.session_id)
            .map_err(|_| AuthError::Internal)?;
        Ok(super::models::AuthenticatedSession {
            account: redeemed.account,
            tokens: super::models::TokenSet {
                access_token,
                access_expires_in: ACCESS_TOKEN_TTL_SECONDS,
                refresh_token,
                refresh_expires_at: redeemed.refresh_expires_at,
            },
        })
    }

    pub async fn refresh_token(&self, token: &str) -> Result<super::models::TokenSet, AuthError> {
        if !(32..=256).contains(&token.len()) {
            return Err(AuthError::Unauthorized);
        }
        let now = Utc::now();
        let refresh_token = self.token_service.create_secret();
        let rotated = self
            .repository
            .rotate_refresh_token(
                self.token_service.hash_secret(token),
                self.token_service.hash_secret(&refresh_token),
                now,
                now + chrono::Duration::days(REFRESH_IDLE_DAYS),
            )
            .await?;
        let access_token = self
            .token_service
            .issue_access_token(rotated.user_id, rotated.session_id)
            .map_err(|_| AuthError::Internal)?;
        Ok(super::models::TokenSet {
            access_token,
            access_expires_in: ACCESS_TOKEN_TTL_SECONDS,
            refresh_token,
            refresh_expires_at: rotated.refresh_expires_at,
        })
    }

    pub(crate) fn access_principal(
        &self,
        token: &str,
    ) -> Result<super::models::AccessPrincipal, AuthError> {
        self.token_service
            .verify_access_token(token)
            .ok_or(AuthError::Unauthorized)
    }

    pub async fn current_account(&self, token: &str) -> Result<super::models::Account, AuthError> {
        let claims = self
            .token_service
            .verify_access_token(token)
            .ok_or(AuthError::Unauthorized)?;
        let user_id = claims.user_id;
        self.repository
            .find_active_account(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)
    }

    pub async fn revoke_current_session(&self, token: &str) -> Result<(), AuthError> {
        if token.len() < 32 || token.len() > 256 {
            return Err(AuthError::Unauthorized);
        }
        if self
            .repository
            .revoke_refresh_session(self.token_service.hash_secret(token))
            .await?
        {
            Ok(())
        } else {
            Err(AuthError::Unauthorized)
        }
    }

    pub async fn request_magic_link(
        &self,
        email: &str,
        ip: &str,
        time_zone: Option<&str>,
    ) -> Result<(), AuthError> {
        let email = normalized_email(email).ok_or(AuthError::InvalidRequest)?;
        let time_zone = time_zone
            .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
            .map(|zone| {
                let zone_name = zone.to_string();
                crate::time_zones::canonicalize_legacy_alias(&zone_name).to_owned()
            })
            .unwrap_or_else(|| "UTC".to_owned());
        let secret = self.token_service.create_secret();
        let challenge_id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::minutes(MAGIC_LINK_TTL_MINUTES);
        self.repository
            .create_magic_link_challenge(
                challenge_id,
                &email,
                ip,
                &time_zone,
                self.token_service.hash_secret(&secret),
                expires_at,
            )
            .await?;
        let link = format!("{}#token={secret}", self.public_url.trim_end_matches('/'));
        if self.mailer.send_magic_link(&email, &link).await.is_err() {
            let _ = self
                .repository
                .consume_magic_link_challenge(challenge_id)
                .await;
            return Err(AuthError::EmailDelivery);
        }
        Ok(())
    }
}

pub(super) fn normalized_email(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() || email.len() > 320 || email.chars().any(char::is_whitespace) {
        return None;
    }
    let parsed = email.parse::<lettre::Address>().ok()?;
    Some(parsed.to_string().to_ascii_lowercase())
}
