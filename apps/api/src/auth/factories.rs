use sqlx::PgPool;
use std::{env, sync::Arc};

use super::{
    email::{EmailSender, SmtpEmailSender},
    jwt_tokens::JwtTokenService,
    repositories::{AuthRepository, PostgresAuthRepository},
    services::AuthApplication,
    token_service::TokenService,
};

pub struct AuthSettings {
    pub jwt_secret: String,
    pub issuer: String,
    pub audience: String,
    pub key_id: String,
    pub public_url: String,
}

impl AuthSettings {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret = env::var("AUTH_JWT_SECRET")
            .map_err(|_| "AUTH_JWT_SECRET must be configured".to_owned())?;
        if jwt_secret.len() < 32 {
            return Err("AUTH_JWT_SECRET must contain at least 32 bytes".to_owned());
        }
        Ok(Self {
            jwt_secret,
            issuer: env::var("AUTH_JWT_ISSUER")
                .map_err(|_| "AUTH_JWT_ISSUER must be configured".to_owned())?,
            audience: env::var("AUTH_JWT_AUDIENCE")
                .map_err(|_| "AUTH_JWT_AUDIENCE must be configured".to_owned())?,
            key_id: env::var("AUTH_JWT_KID").unwrap_or_else(|_| "v1".to_owned()),
            public_url: env::var("AUTH_PUBLIC_URL")
                .map_err(|_| "AUTH_PUBLIC_URL must be configured".to_owned())?,
        })
    }
}

impl AuthApplication {
    pub fn from_env(pool: PgPool) -> Result<Self, String> {
        let settings = AuthSettings::from_env()?;
        Self::with_dependencies(
            Arc::new(SmtpEmailSender::from_env()?),
            Arc::new(PostgresAuthRepository::new(pool)),
            settings,
        )
    }

    pub fn with_email_sender(
        pool: PgPool,
        mailer: Arc<dyn EmailSender>,
        settings: AuthSettings,
    ) -> Result<Self, String> {
        let repository = Arc::new(PostgresAuthRepository::new(pool));
        Self::with_dependencies(mailer, repository, settings)
    }

    pub fn with_dependencies(
        mailer: Arc<dyn EmailSender>,
        repository: Arc<dyn AuthRepository>,
        settings: AuthSettings,
    ) -> Result<Self, String> {
        if settings.jwt_secret.len() < 32 {
            return Err("AUTH_JWT_SECRET must contain at least 32 bytes".to_owned());
        }
        let token_service = Arc::new(JwtTokenService::new(
            &settings.jwt_secret,
            settings.issuer,
            settings.audience,
            settings.key_id,
        ));
        Ok(Self {
            mailer,
            token_service,
            public_url: settings.public_url,
            repository,
        })
    }

    pub fn with_services(
        mailer: Arc<dyn EmailSender>,
        repository: Arc<dyn AuthRepository>,
        token_service: Arc<dyn TokenService>,
        settings: AuthSettings,
    ) -> Result<Self, String> {
        if settings.jwt_secret.len() < 32 {
            return Err("AUTH_JWT_SECRET must contain at least 32 bytes".to_owned());
        }
        Ok(Self {
            mailer,
            token_service,
            public_url: settings.public_url,
            repository,
        })
    }
}
