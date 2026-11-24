use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::{
    Account, AuthError, MAGIC_LINK_IP_PER_MINUTE_LIMIT, MAGIC_LINK_RESEND_COOLDOWN_SECONDS,
};
pub struct RedeemedMagicLink {
    pub account: Account,
    pub session_id: Uuid,
    pub refresh_expires_at: DateTime<Utc>,
}

pub struct RotatedRefreshToken {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub refresh_expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RateLimitCounts {
    ip_count: i64,
    latest_email_request: Option<DateTime<Utc>>,
    oldest_ip_request: Option<DateTime<Utc>>,
}

pub(super) async fn lock_rate_limit_keys(
    transaction: &mut Transaction<'_, Postgres>,
    email: &str,
    ip: &str,
) -> Result<(), sqlx::Error> {
    let mut keys = [format!("email:{email}"), format!("ip:{ip}")];
    keys.sort();
    for key in keys {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(key)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

pub(super) async fn is_rate_limited(
    transaction: &mut Transaction<'_, Postgres>,
    email: &str,
    ip: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let rate_limits: RateLimitCounts = sqlx::query_as(
        "SELECT email_requests.latest_request AS latest_email_request, \
                    ip_requests.request_count AS ip_count, \
                    ip_requests.oldest_request AS oldest_ip_request \
             FROM ( \
                 SELECT MAX(created_at) AS latest_request \
                 FROM auth_challenges \
                 WHERE email_normalized = $1 \
             ) AS email_requests \
             CROSS JOIN ( \
                 SELECT COUNT(*) AS request_count, MIN(created_at) AS oldest_request \
                 FROM auth_challenges \
                 WHERE request_ip = $2 \
                   AND created_at > now() - interval '1 minute' \
             ) AS ip_requests",
    )
    .bind(email)
    .bind(ip)
    .fetch_one(&mut **transaction)
    .await?;

    let RateLimitCounts {
        ip_count,
        latest_email_request,
        oldest_ip_request,
    } = rate_limits;
    let now = Utc::now();
    let mut retry_after_seconds = 0;
    let mut include_expiry = |expiry: Option<DateTime<Utc>>| {
        if let Some(expiry) = expiry {
            let remaining_milliseconds = (expiry - now).num_milliseconds();
            if remaining_milliseconds > 0 {
                retry_after_seconds =
                    retry_after_seconds.max((remaining_milliseconds + 999) / 1_000);
            }
        }
    };
    if latest_email_request.is_some_and(|latest| {
        latest > now - ChronoDuration::seconds(MAGIC_LINK_RESEND_COOLDOWN_SECONDS)
    }) {
        include_expiry(
            latest_email_request
                .map(|latest| latest + ChronoDuration::seconds(MAGIC_LINK_RESEND_COOLDOWN_SECONDS)),
        );
    }
    if ip_count >= MAGIC_LINK_IP_PER_MINUTE_LIMIT {
        include_expiry(oldest_ip_request.map(|oldest| oldest + ChronoDuration::minutes(1)));
    }
    Ok((retry_after_seconds > 0).then_some(retry_after_seconds.max(1)))
}

pub enum SignInProof {
    Link(Vec<u8>),
    Code { challenge: Uuid, hash: Vec<u8> },
}

#[derive(sqlx::FromRow)]
struct SignInChallenge {
    id: Uuid,
    email_normalized: String,
    time_zone: String,
    code_hash: Option<Vec<u8>>,
    code_attempts: i32,
}

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn create_magic_link_challenge(
        &self,
        id: Uuid,
        email: &str,
        ip: &str,
        time_zone: &str,
        hashes: (Vec<u8>, Vec<u8>),
        expires_at: DateTime<Utc>,
    ) -> Result<(), super::models::AuthError>;

    async fn consume_magic_link_challenge(&self, id: Uuid) -> Result<(), super::models::AuthError>;

    async fn find_active_account(
        &self,
        user_id: Uuid,
    ) -> Result<Option<super::models::Account>, super::models::AuthError>;

    async fn revoke_refresh_session(
        &self,
        secret_hash: Vec<u8>,
    ) -> Result<bool, super::models::AuthError>;

    async fn redeem_magic_link(
        &self,
        proof: SignInProof,
        session_id: Uuid,
        refresh_hash: Vec<u8>,
        now: DateTime<Utc>,
        idle_expires_at: DateTime<Utc>,
        absolute_expires_at: DateTime<Utc>,
    ) -> Result<RedeemedMagicLink, AuthError>;

    async fn rotate_refresh_token(
        &self,
        presented_hash: Vec<u8>,
        replacement_hash: Vec<u8>,
        now: DateTime<Utc>,
        requested_expiry: DateTime<Utc>,
    ) -> Result<RotatedRefreshToken, AuthError>;
}

pub struct PostgresAuthRepository {
    pool: PgPool,
}

impl PostgresAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn create_magic_link_challenge(
        &self,
        id: Uuid,
        email: &str,
        ip: &str,
        time_zone: &str,
        hashes: (Vec<u8>, Vec<u8>),
        expires_at: DateTime<Utc>,
    ) -> Result<(), super::models::AuthError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| super::models::AuthError::Unavailable)?;
        lock_rate_limit_keys(&mut transaction, email, ip)
            .await
            .map_err(|_| super::models::AuthError::Unavailable)?;
        match is_rate_limited(&mut transaction, email, ip).await {
            Ok(Some(retry_after_seconds)) => {
                return Err(super::models::AuthError::RateLimited {
                    retry_after_seconds,
                });
            }
            Ok(None) => {}
            Err(_) => return Err(super::models::AuthError::Unavailable),
        }
        sqlx::query(
            "INSERT INTO auth_challenges (id, email_normalized, secret_hash, request_ip, time_zone, expires_at, code_hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(email)
        .bind(hashes.0)
        .bind(ip)
        .bind(time_zone)
        .bind(expires_at)
        .bind(hashes.1)
        .execute(&mut *transaction)
        .await
        .map_err(|_| super::models::AuthError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| super::models::AuthError::Unavailable)
    }

    async fn consume_magic_link_challenge(&self, id: Uuid) -> Result<(), super::models::AuthError> {
        sqlx::query("UPDATE auth_challenges SET consumed_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| super::models::AuthError::Unavailable)
    }

    async fn find_active_account(
        &self,
        user_id: Uuid,
    ) -> Result<Option<super::models::Account>, super::models::AuthError> {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT users.id, email_identities.email_normalized \
             FROM users JOIN email_identities ON email_identities.user_id = users.id \
             WHERE users.id = $1 AND users.status = 'active' \
             ORDER BY email_identities.created_at LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|(id, email)| super::models::Account { id, email }))
        .map_err(|error| {
            log::error!(target: "horizon_api::database", "auth.find_active_account failed: {error}");
            super::models::AuthError::Unavailable
        })
    }

    async fn revoke_refresh_session(
        &self,
        secret_hash: Vec<u8>,
    ) -> Result<bool, super::models::AuthError> {
        sqlx::query(
            "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, now()) \
             WHERE id = (SELECT session_id FROM refresh_tokens WHERE secret_hash = $1)",
        )
        .bind(secret_hash)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|_| super::models::AuthError::Unavailable)
    }

    async fn redeem_magic_link(
        &self,
        proof: SignInProof,
        session_id: Uuid,
        refresh_hash: Vec<u8>,
        now: DateTime<Utc>,
        idle_expires_at: DateTime<Utc>,
        absolute_expires_at: DateTime<Utc>,
    ) -> Result<RedeemedMagicLink, AuthError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::Unavailable)?;
        let (challenge_id, link_hash) = match &proof {
            SignInProof::Link(hash) => (None, Some(hash.as_slice())),
            SignInProof::Code { challenge, .. } => (Some(*challenge), None),
        };
        let challenge = sqlx::query_as::<_, SignInChallenge>(
            "SELECT id, email_normalized, time_zone, code_hash, code_attempts \
             FROM auth_challenges WHERE (id = $1 OR secret_hash = $2) \
               AND consumed_at IS NULL AND expires_at > $3 FOR UPDATE",
        )
        .bind(challenge_id)
        .bind(link_hash)
        .bind(now)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        let Some(challenge) = challenge else {
            return Err(AuthError::Unauthorized);
        };
        if let SignInProof::Code { hash, .. } = proof {
            let Some(stored) = challenge.code_hash else {
                return Err(AuthError::Unauthorized);
            };
            if challenge.code_attempts >= 5 {
                return Err(AuthError::Unauthorized);
            }
            if stored.len() != hash.len() || !openssl::memcmp::eq(&stored, &hash) {
                sqlx::query(
                    "UPDATE auth_challenges SET code_attempts = code_attempts + 1 WHERE id = $1",
                )
                .bind(challenge.id)
                .execute(&mut *transaction)
                .await
                .map_err(|_| AuthError::Unavailable)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| AuthError::Unavailable)?;
                return Err(AuthError::Unauthorized);
            }
        }
        let challenge_id = challenge.id;
        let email = challenge.email_normalized;
        let time_zone = challenge.time_zone;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("account:{email}"))
            .execute(&mut *transaction)
            .await
            .map_err(|_| AuthError::Unavailable)?;
        let consumed = sqlx::query(
            "UPDATE auth_challenges SET consumed_at = now() \
             WHERE id = $1 AND consumed_at IS NULL",
        )
        .bind(challenge_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        if consumed.rows_affected() != 1 {
            return Err(AuthError::Unauthorized);
        }

        let existing_user = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT users.id, users.status FROM email_identities \
             JOIN users ON users.id = email_identities.user_id \
             WHERE email_identities.email_normalized = $1 FOR UPDATE OF users",
        )
        .bind(&email)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        let user_id = match existing_user {
            Some((id, status)) if status == "active" => id,
            Some(_) => return Err(AuthError::Unauthorized),
            None => {
                let user_id = Uuid::new_v4();
                let identity_id = Uuid::new_v4();
                sqlx::query("INSERT INTO users (id, time_zone) VALUES ($1, $2)")
                    .bind(user_id)
                    .bind(time_zone)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| AuthError::Unavailable)?;
                sqlx::query(
                    "INSERT INTO email_identities (id, user_id, email_normalized, verified_at) \
                     VALUES ($1, $2, $3, now())",
                )
                .bind(identity_id)
                .bind(user_id)
                .bind(&email)
                .execute(&mut *transaction)
                .await
                .map_err(|_| AuthError::Unavailable)?;
                let space_id = Uuid::new_v4();
                if let Some(space_id) = sqlx::query_scalar::<_, Uuid>(
                    "INSERT INTO spaces (id, name, kind, personal_owner_id) \
                     VALUES ($1, 'Personal', 'personal', $2) \
                     ON CONFLICT (personal_owner_id) DO NOTHING RETURNING id",
                )
                .bind(space_id)
                .bind(user_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|_| AuthError::Unavailable)?
                {
                    sqlx::query(
                        "INSERT INTO space_memberships (space_id, user_id, role) \
                         VALUES ($1, $2, 'owner')",
                    )
                    .bind(space_id)
                    .bind(user_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| AuthError::Unavailable)?;
                }
                user_id
            }
        };

        sqlx::query(
            "INSERT INTO auth_sessions (id, user_id, idle_expires_at, absolute_expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(idle_expires_at)
        .bind(absolute_expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        sqlx::query(
            "INSERT INTO refresh_tokens (id, session_id, secret_hash, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(refresh_hash)
        .bind(idle_expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthError::Unavailable)?;
        Ok(RedeemedMagicLink {
            account: Account { id: user_id, email },
            session_id,
            refresh_expires_at: idle_expires_at,
        })
    }

    async fn rotate_refresh_token(
        &self,
        presented_hash: Vec<u8>,
        replacement_hash: Vec<u8>,
        now: DateTime<Utc>,
        requested_expiry: DateTime<Utc>,
    ) -> Result<RotatedRefreshToken, AuthError> {
        let session_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT session_id FROM refresh_tokens WHERE secret_hash = $1",
        )
        .bind(&presented_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::Unavailable)?
        .ok_or(AuthError::Unauthorized)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::Unavailable)?;
        let session =
            sqlx::query_as::<_, (Uuid, DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>)>(
                "SELECT user_id, idle_expires_at, absolute_expires_at, revoked_at \
             FROM auth_sessions WHERE id = $1 FOR UPDATE",
            )
            .bind(session_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| AuthError::Unavailable)?
            .ok_or(AuthError::Unauthorized)?;
        let (user_id, idle_expiry, absolute_expiry, revoked_at) = session;
        let refresh = sqlx::query_as::<_, (Uuid, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT id, expires_at, consumed_at FROM refresh_tokens \
             WHERE session_id = $1 AND secret_hash = $2 FOR UPDATE",
        )
        .bind(session_id)
        .bind(&presented_hash)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?
        .ok_or(AuthError::Unauthorized)?;
        let (refresh_id, token_expiry, consumed_at) = refresh;
        if consumed_at.is_some() {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, now()) WHERE id = $1",
            )
            .bind(session_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| AuthError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| AuthError::Unavailable)?;
            return Err(AuthError::Unauthorized);
        }
        if revoked_at.is_some()
            || token_expiry <= now
            || idle_expiry <= now
            || absolute_expiry <= now
        {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, now()) WHERE id = $1",
            )
            .bind(session_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| AuthError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| AuthError::Unavailable)?;
            return Err(AuthError::Unauthorized);
        }
        let new_expiry = requested_expiry.min(absolute_expiry);
        sqlx::query(
            "UPDATE refresh_tokens SET consumed_at = now() WHERE id = $1 AND consumed_at IS NULL",
        )
        .bind(refresh_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        sqlx::query(
            "INSERT INTO refresh_tokens (id, session_id, secret_hash, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(replacement_hash)
        .bind(new_expiry)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        sqlx::query(
            "UPDATE auth_sessions SET last_used_at = $2, idle_expires_at = $3 WHERE id = $1",
        )
        .bind(session_id)
        .bind(now)
        .bind(new_expiry)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthError::Unavailable)?;
        Ok(RotatedRefreshToken {
            user_id,
            session_id,
            refresh_expires_at: new_expiry,
        })
    }
}
