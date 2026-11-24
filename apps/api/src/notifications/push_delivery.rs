use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, NaiveDate, Utc};
use hyper::{client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use sqlx::PgPool;
use uuid::Uuid;
use web_push::{
    request_builder, ContentEncoding, SubscriptionInfo, VapidSignatureBuilder,
    WebPushMessageBuilder,
};

fn config() -> Option<(String, String, String)> {
    let public = std::env::var("WEB_PUSH_PUBLIC_KEY").ok()?;
    let private = std::env::var("WEB_PUSH_PRIVATE_KEY").ok()?;
    let subject = std::env::var("WEB_PUSH_SUBJECT").ok()?;
    let contact = subject.strip_prefix("mailto:")?;
    if !contact.contains('@') || contact.chars().any(char::is_whitespace) {
        return None;
    }
    let signature = VapidSignatureBuilder::from_base64_no_sub(&private).ok()?;
    (URL_SAFE_NO_PAD.encode(signature.get_public_key()) == public)
        .then_some((public, private, subject))
}

pub struct PushWorker {
    pool: PgPool,
    private_key: String,
    subject: String,
    client: Client<HttpsConnector<HttpConnector>>,
}

impl PushWorker {
    pub fn from_env(pool: PgPool) -> Option<Self> {
        let (_, private_key, subject) = config()?;
        let mut connector = HttpsConnector::new();
        connector.https_only(true);
        Some(Self {
            pool,
            private_key,
            subject,
            client: Client::builder().build(connector),
        })
    }

    pub fn public_key() -> Option<String> {
        config().map(|(public, _, _)| public)
    }

    pub async fn run_once(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM web_push_outbox WHERE expires_at < now() - interval '1 day'")
            .execute(&self.pool)
            .await?;
        for _ in 0..50 {
            let job = sqlx::query_as::<_, PushJob>(
                "WITH claimed AS ( \
                     UPDATE web_push_outbox SET attempts = attempts + 1, \
                         available_at = now() + interval '2 minutes' \
                     WHERE id = (SELECT id FROM web_push_outbox \
                         WHERE NOT finished AND attempts < 3 AND expires_at > now() \
                           AND available_at <= now() \
                         ORDER BY available_at, id FOR UPDATE SKIP LOCKED LIMIT 1) \
                     RETURNING * \
                 ) SELECT j.id, j.endpoint, j.space_id, j.kind, j.item_id, j.occurrence_date, \
                          j.reference_time, j.offset_minutes, j.expires_at, \
                          s.user_id, s.p256dh, s.auth_secret, \
                          ((CASE WHEN j.kind = 'calendar' THEN s.calendar_reminders ELSE s.task_reminders END) \
                           AND EXISTS (SELECT 1 FROM auth_sessions a WHERE a.id = s.session_id \
                               AND a.user_id = s.user_id AND a.revoked_at IS NULL AND a.absolute_expires_at > now()) \
                           AND EXISTS (SELECT 1 FROM spaces sp JOIN space_memberships m ON m.space_id = sp.id \
                               WHERE sp.id = j.space_id AND sp.deleted_at IS NULL AND m.user_id = s.user_id) \
                           AND NOT EXISTS (SELECT 1 FROM space_notification_preferences \
                               WHERE user_id = s.user_id AND space_id = j.space_id AND muted)) AS enabled \
                 FROM claimed j JOIN web_push_subscriptions s ON s.endpoint = j.endpoint",
            )
            .fetch_optional(&self.pool)
            .await?;
            let Some(job) = job else { break };
            let current = if !job.enabled {
                false
            } else if job.kind == "calendar" {
                crate::events::push_reminder_is_current(
                    &self.pool,
                    job.user_id,
                    job.item_id,
                    job.occurrence_date,
                    job.reference_time,
                    job.offset_minutes,
                )
                .await?
            } else {
                crate::task_reminders::push_reminder_is_current(
                    &self.pool,
                    job.user_id,
                    job.item_id,
                    job.reference_time,
                    job.offset_minutes,
                )
                .await?
            };
            match if current {
                self.send(&job).await
            } else {
                Outcome::Finished
            } {
                Outcome::Finished => {
                    sqlx::query("UPDATE web_push_outbox SET finished = TRUE WHERE id = $1")
                        .bind(job.id)
                        .execute(&self.pool)
                        .await?;
                }
                Outcome::Gone => {
                    sqlx::query("DELETE FROM web_push_subscriptions WHERE endpoint = $1 AND user_id = $2 AND p256dh = $3 AND auth_secret = $4")
                        .bind(&job.endpoint)
                        .bind(job.user_id)
                        .bind(&job.p256dh)
                        .bind(&job.auth_secret)
                        .execute(&self.pool)
                        .await?;
                }
                Outcome::Retry => {}
            }
        }
        Ok(())
    }

    async fn send(&self, job: &PushJob) -> Outcome {
        let Some(request) = self.build_request(job) else {
            return Outcome::Finished;
        };
        // Hyper never follows redirects. Don't log requests or provider bodies.
        let Ok(Ok(response)) = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            self.client.request(request),
        )
        .await
        else {
            return Outcome::Retry;
        };
        match response.status().as_u16() {
            404 | 410 => Outcome::Gone,
            408 | 429 | 500..=599 => Outcome::Retry,
            200..=299 => Outcome::Finished,
            code => {
                log::warn!("Web Push provider rejected a reminder (HTTP {code})");
                Outcome::Finished
            }
        }
    }

    fn build_request(&self, job: &PushJob) -> Option<hyper::Request<Body>> {
        let ttl = (job.expires_at - Utc::now()).num_seconds();
        if ttl <= 0 || !super::push::supported_endpoint(&job.endpoint) {
            return None;
        }
        let subscription = SubscriptionInfo::new(
            job.endpoint.clone(),
            URL_SAFE_NO_PAD.encode(&job.p256dh),
            URL_SAFE_NO_PAD.encode(&job.auth_secret),
        );
        let mut vapid =
            VapidSignatureBuilder::from_base64(&self.private_key, &subscription).ok()?;
        vapid.add_claim("sub", self.subject.clone());
        let signature = vapid.build().ok()?;
        let mut builder = WebPushMessageBuilder::new(&subscription);
        builder.set_vapid_signature(signature);
        builder.set_ttl(ttl as u32);
        let payload = serde_json::json!({
            "kind": job.kind,
            "space": job.space_id,
            "item": job.item_id,
            "occurrence": job.occurrence_date,
            "tag": job.id,
        })
        .to_string();
        builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
        Some(request_builder::build_request(builder.build().ok()?))
    }
}

enum Outcome {
    Finished,
    Gone,
    Retry,
}

#[derive(sqlx::FromRow)]
struct PushJob {
    id: Uuid,
    endpoint: String,
    space_id: Uuid,
    kind: String,
    item_id: Uuid,
    occurrence_date: Option<NaiveDate>,
    reference_time: DateTime<Utc>,
    offset_minutes: i32,
    expires_at: DateTime<Utc>,
    user_id: Uuid,
    p256dh: Vec<u8>,
    auth_secret: Vec<u8>,
    enabled: bool,
}
