use std::sync::Arc;

use chrono::{Days, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use sqlx::{postgres::PgPool, FromRow};
use uuid::Uuid;

use crate::auth::EmailSender;

const RECONCILE_BATCH_SIZE: i64 = 256;
const DELIVERY_BATCH_SIZE: usize = 50;

#[derive(Clone)]
pub struct TaskReminderWorker {
    pool: PgPool,
    email: Arc<dyn EmailSender>,
}

impl TaskReminderWorker {
    pub fn new(pool: PgPool, email: Arc<dyn EmailSender>) -> Self {
        Self { pool, email }
    }

    /// Reconciles durable jobs from current Task state, then drains due work.
    pub async fn run_once(&self) -> Result<(), sqlx::Error> {
        self.run_once_at(Utc::now()).await
    }

    /// Drains work against an explicit clock, useful for deterministic worker tests.
    pub async fn run_once_at(&self, now: chrono::DateTime<Utc>) -> Result<(), sqlx::Error> {
        self.reconcile_pending().await?;
        self.deliver_due_at(now).await?;
        Ok(())
    }

    pub async fn reconcile_pending(&self) -> Result<(), sqlx::Error> {
        self.reconcile(Utc::now()).await
    }

    /// Claim one job at a time so a slow send does not tie up a batch of leases.
    pub async fn deliver_due_at(&self, now: chrono::DateTime<Utc>) -> Result<usize, sqlx::Error> {
        for claimed in 0..DELIVERY_BATCH_SIZE {
            let Some(job) = self.claim(now).await? else {
                return Ok(claimed);
            };
            self.deliver(job).await;
        }
        Ok(DELIVERY_BATCH_SIZE)
    }

    async fn reconcile(&self, now: chrono::DateTime<Utc>) -> Result<(), sqlx::Error> {
        let mut after = None;
        while let Some(last) = self.cancel_batch(after).await? {
            after = Some(last);
        }

        let mut after = None;
        loop {
            let page = self.candidate_batch(after).await?;
            let Some(last) = page.last() else {
                return Ok(());
            };
            // An ineligible Task still advances the raw page's cursor.
            after = Some(last.task_id);
            self.upsert_batch(&page, now).await?;
        }
    }

    async fn cancel_batch(&self, after: Option<Uuid>) -> Result<Option<Uuid>, sqlx::Error> {
        // Advance over checked jobs, not just cancelled jobs, so valid jobs
        // don't pin the cursor. The update and cursor selection share a snapshot.
        sqlx::query_scalar(
            "WITH batch AS MATERIALIZED ( \
               SELECT id FROM task_reminder_jobs WHERE status = 'pending' \
               AND id >= $1 AND ($2 OR id > $1) ORDER BY id LIMIT $3 \
              ), cancelled AS ( \
              UPDATE task_reminder_jobs j SET status = 'cancelled', locked_at = NULL \
              FROM batch WHERE j.id = batch.id AND j.status = 'pending' AND NOT EXISTS ( \
                SELECT 1 FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
                JOIN spaces s ON s.id = l.space_id AND s.deleted_at IS NULL \
                JOIN space_memberships m ON m.space_id = s.id AND m.user_id = j.recipient_id \
                JOIN users u ON u.id = COALESCE(t.assignee_id, t.creator_id) \
               LEFT JOIN task_reminder_preferences p \
                 ON p.task_id = t.id AND p.user_id = u.id \
               WHERE t.id = j.task_id AND NOT t.done AND t.due_date IS NOT NULL \
                 AND COALESCE(t.assignee_id, t.creator_id) = j.recipient_id \
                 AND NOT EXISTS (SELECT 1 FROM space_notification_preferences mute \
                   WHERE mute.user_id = j.recipient_id AND mute.space_id = l.space_id AND mute.muted) \
                 AND (COALESCE((SELECT enabled FROM notification_preferences \
                      WHERE user_id = j.recipient_id AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) \
                   OR COALESCE((SELECT enabled FROM notification_preferences \
                       WHERE user_id = j.recipient_id AND event_type = 'due_reminder' AND channel = 'email'), FALSE) \
                    OR EXISTS (SELECT 1 FROM web_push_subscriptions WHERE user_id = j.recipient_id AND task_reminders)) \
                 AND j.offset_minutes = ANY(COALESCE(p.offsets_minutes, t.reminder_offsets_minutes)) \
              ) RETURNING j.id \
              ) SELECT id FROM batch ORDER BY id DESC LIMIT 1",
        )
        .bind(after.unwrap_or_default())
        .bind(after.is_none())
        .bind(RECONCILE_BATCH_SIZE)
        .fetch_optional(&self.pool)
        .await
    }

    async fn candidate_batch(
        &self,
        after: Option<Uuid>,
    ) -> Result<Vec<ReminderCandidate>, sqlx::Error> {
        sqlx::query_as::<_, ReminderCandidate>(
            "WITH batch AS MATERIALIZED ( \
               SELECT id, list_id, creator_id, assignee_id, due_date, due_time, reminder_offsets_minutes \
               FROM tasks WHERE NOT done AND due_date IS NOT NULL \
                 AND id >= $1 AND ($2 OR id > $1) ORDER BY id LIMIT $3 \
             ), eligible AS ( \
              SELECT t.id, u.id AS recipient_id, u.time_zone, u.default_reminder_time, \
                 COALESCE(p.offsets_minutes, t.reminder_offsets_minutes) AS offsets_minutes \
              FROM batch t JOIN todo_lists l ON l.id = t.list_id \
              JOIN spaces s ON s.id = l.space_id AND s.deleted_at IS NULL \
              JOIN users u ON u.id = COALESCE(t.assignee_id, t.creator_id) \
              JOIN space_memberships m ON m.space_id = s.id AND m.user_id = u.id \
             LEFT JOIN task_reminder_preferences p ON p.task_id = t.id AND p.user_id = u.id \
              WHERE NOT EXISTS (SELECT 1 FROM space_notification_preferences mute \
                 WHERE mute.user_id = u.id AND mute.space_id = l.space_id AND mute.muted) \
               AND (COALESCE((SELECT enabled FROM notification_preferences \
                    WHERE user_id = u.id AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) \
                 OR COALESCE((SELECT enabled FROM notification_preferences \
                     WHERE user_id = u.id AND event_type = 'due_reminder' AND channel = 'email'), FALSE) \
                   OR EXISTS (SELECT 1 FROM web_push_subscriptions WHERE user_id = u.id AND task_reminders)) \
             ) SELECT b.id AS task_id, e.recipient_id, e.offsets_minutes, \
                      b.due_date, b.due_time, e.time_zone, e.default_reminder_time \
               FROM batch b LEFT JOIN eligible e ON e.id = b.id ORDER BY b.id",
        )
        .bind(after.unwrap_or_default())
        .bind(after.is_none())
        .bind(RECONCILE_BATCH_SIZE)
        .fetch_all(&self.pool)
        .await
    }

    async fn upsert_batch(
        &self,
        batch: &[ReminderCandidate],
        now: chrono::DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut task_ids = Vec::new();
        let mut recipients = Vec::new();
        let mut offsets = Vec::new();
        let mut schedules = Vec::new();
        for candidate in batch {
            let (Some(recipient_id), Some(candidate_offsets)) =
                (candidate.recipient_id, &candidate.offsets_minutes)
            else {
                continue;
            };
            for &offset in candidate_offsets {
                let Some(scheduled_at) = candidate.scheduled_at(offset) else {
                    continue;
                };
                if scheduled_at > now {
                    task_ids.push(candidate.task_id);
                    recipients.push(recipient_id);
                    offsets.push(offset);
                    schedules.push(scheduled_at);
                }
            }
        }
        if task_ids.is_empty() {
            return Ok(());
        }
        sqlx::query(
                "INSERT INTO task_reminder_jobs \
                 (id, task_id, recipient_id, offset_minutes, scheduled_at, available_at) \
                 SELECT gen_random_uuid(), candidate.task_id, candidate.recipient_id, \
                        candidate.offset_minutes, candidate.scheduled_at, candidate.scheduled_at \
                 FROM (SELECT DISTINCT * FROM unnest($1::uuid[], $2::uuid[], $3::int[], $4::timestamptz[]) \
                    AS job(task_id, recipient_id, offset_minutes, scheduled_at)) candidate \
                 WHERE NOT EXISTS (SELECT 1 FROM task_reminder_jobs j \
                    WHERE (j.task_id, j.recipient_id, j.offset_minutes) \
                        = (candidate.task_id, candidate.recipient_id, candidate.offset_minutes) \
                      AND (j.status IN ('sending', 'sent', 'failed') \
                        OR (j.status = 'pending' AND j.scheduled_at = candidate.scheduled_at))) \
                 ORDER BY candidate.task_id, candidate.recipient_id, candidate.offset_minutes \
                 ON CONFLICT (task_id, recipient_id, offset_minutes) DO UPDATE \
                    SET status = 'pending', sent_at = NULL, last_error = NULL, \
                        scheduled_at = EXCLUDED.scheduled_at, \
                        available_at = CASE WHEN task_reminder_jobs.scheduled_at IS DISTINCT FROM EXCLUDED.scheduled_at \
                          THEN EXCLUDED.available_at ELSE task_reminder_jobs.available_at END \
                    WHERE task_reminder_jobs.status = 'cancelled' \
                       OR (task_reminder_jobs.status = 'pending' \
                         AND task_reminder_jobs.scheduled_at IS DISTINCT FROM EXCLUDED.scheduled_at)",
            )
            .bind(&task_ids)
            .bind(&recipients)
            .bind(&offsets)
            .bind(&schedules)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn claim(&self, now: chrono::DateTime<Utc>) -> Result<Option<ReminderJob>, sqlx::Error> {
        sqlx::query_as::<_, ReminderJob>(
            "UPDATE task_reminder_jobs SET status = 'sending', locked_at = now(), attempts = attempts + 1 \
             WHERE id = (SELECT j.id FROM task_reminder_jobs j \
               WHERE j.status = 'pending' AND j.scheduled_at <= $1 AND j.available_at <= $1 \
                 AND EXISTS (SELECT 1 FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
                   JOIN spaces s ON s.id = l.space_id AND s.deleted_at IS NULL \
                   JOIN space_memberships m ON m.space_id = s.id AND m.user_id = j.recipient_id \
                   WHERE t.id = j.task_id) \
               ORDER BY j.scheduled_at, j.id FOR UPDATE OF j SKIP LOCKED LIMIT 1) \
             RETURNING id, task_id, recipient_id, offset_minutes, attempts",
        )
        .bind(now)
        .fetch_optional(&self.pool)
        .await
    }

    async fn deliver(&self, job: ReminderJob) {
        let result = self.deliver_claimed(&job).await;
        match result {
            Ok(delivered) => {
                let _ = sqlx::query(
                    "UPDATE task_reminder_jobs SET status = $2, \
                     sent_at = CASE WHEN $2 = 'sent' THEN now() END, locked_at = NULL, last_error = NULL \
                     WHERE id = $1 AND status = 'sending' AND attempts = $3",
                )
                .bind(job.id)
                .bind(if delivered { "sent" } else { "cancelled" })
                .bind(job.attempts)
                .execute(&self.pool)
                .await;
            }
            Err(error) => {
                let terminal = job.attempts >= 8;
                let delay_seconds =
                    60_i64.saturating_mul(1_i64 << (job.attempts.saturating_sub(1).min(10)));
                let _ = sqlx::query(
                    "UPDATE task_reminder_jobs SET status = $2, locked_at = NULL, \
                      available_at = now() + ($3 * interval '1 second'), last_error = $4 \
                      WHERE id = $1 AND status = 'sending' AND attempts = $5",
                )
                .bind(job.id)
                .bind(if terminal { "failed" } else { "pending" })
                .bind(delay_seconds)
                .bind(error)
                .bind(job.attempts)
                .execute(&self.pool)
                .await;
            }
        }
    }

    async fn deliver_claimed(&self, job: &ReminderJob) -> Result<bool, String> {
        let row = sqlx::query_as::<_, DeliveryRow>(
            "SELECT t.title, s.id AS space_id, t.due_date, t.due_time, \
               u.time_zone, u.default_reminder_time, j.scheduled_at, \
               e.email_normalized AS email, \
               COALESCE((SELECT enabled FROM notification_preferences WHERE user_id = j.recipient_id \
                  AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) AS in_app, \
               COALESCE((SELECT enabled FROM notification_preferences WHERE user_id = j.recipient_id \
                  AND event_type = 'due_reminder' AND channel = 'email'), FALSE) AS email_enabled, \
               EXISTS (SELECT 1 FROM space_notification_preferences WHERE user_id = j.recipient_id \
                  AND space_id = l.space_id AND muted) AS muted \
              FROM task_reminder_jobs j JOIN tasks t ON t.id = j.task_id \
              JOIN todo_lists l ON l.id = t.list_id JOIN spaces s ON s.id = l.space_id \
              JOIN space_memberships m ON m.space_id = s.id AND m.user_id = j.recipient_id \
             JOIN users u ON u.id = j.recipient_id \
             JOIN LATERAL (SELECT email_normalized FROM email_identities \
               WHERE user_id = j.recipient_id ORDER BY created_at LIMIT 1) e ON TRUE \
              WHERE j.id = $1 AND NOT t.done AND t.due_date IS NOT NULL AND s.deleted_at IS NULL \
               AND ((t.due_time IS NOT NULL AND ((t.due_date + t.due_time) AT TIME ZONE u.time_zone) > now()) \
                 OR (t.due_time IS NULL AND t.due_date >= (now() AT TIME ZONE u.time_zone)::date)) \
               AND j.recipient_id = COALESCE(t.assignee_id, t.creator_id) \
               AND j.offset_minutes = ANY(COALESCE((SELECT offsets_minutes FROM task_reminder_preferences \
                    WHERE task_id = t.id AND user_id = j.recipient_id), t.reminder_offsets_minutes))",
        )
        .bind(job.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        let Some(row) = row.filter(|row| {
            !row.muted
                && reminder_time(
                    row.due_date,
                    row.due_time,
                    &row.time_zone,
                    row.default_reminder_time,
                    job.offset_minutes,
                ) == Some(row.scheduled_at)
        }) else {
            // Reconciliation may not have caught a schedule change yet. A
            // skipped claim must remain eligible for a new, future schedule.
            return Ok(false);
        };

        let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
        // Match Space deletion's lock order so membership cannot disappear while
        // we write the in-app notification or outbox item.
        let active_space = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM spaces WHERE id = $1 AND deleted_at IS NULL FOR KEY SHARE",
        )
        .bind(row.space_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        let current_member = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM space_memberships WHERE space_id = $1 AND user_id = $2 FOR KEY SHARE",
        )
        .bind(row.space_id)
        .bind(job.recipient_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        if active_space.is_none() || current_member.is_none() {
            return Ok(false);
        }

        sqlx::query(
            "INSERT INTO web_push_outbox \
                 (endpoint, space_id, reminder_key, kind, item_id, reference_time, offset_minutes, expires_at) \
             SELECT endpoint, $2, concat('task:', $3::uuid), 'tasks', $4, $5::timestamptz, $6, $5::timestamptz + interval '10 minutes' \
             FROM web_push_subscriptions WHERE user_id = $1 AND task_reminders \
               AND $5::timestamptz + interval '10 minutes' > now() \
             ON CONFLICT (endpoint, reminder_key) DO NOTHING",
        )
        .bind(job.recipient_id)
        .bind(row.space_id)
        .bind(job.id)
        .bind(job.task_id)
        .bind(row.scheduled_at)
        .bind(job.offset_minutes)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;

        if row.in_app {
            sqlx::query(
                "INSERT INTO notifications (id, recipient_id, space_id, event_type, title, body, destination, reminder_job_id) \
                 SELECT gen_random_uuid(), $1, l.space_id, 'due_reminder', 'Task reminder', $3, 'tasks', $4 \
                 FROM tasks t JOIN todo_lists l ON l.id = t.list_id WHERE t.id = $2 \
                 ON CONFLICT (reminder_job_id) DO NOTHING",
            )
            .bind(job.recipient_id)
            .bind(job.task_id)
            .bind(&row.title)
            .bind(job.id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        }
        tx.commit().await.map_err(|error| error.to_string())?;
        if row.email_enabled {
            let may_email = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM space_memberships m \
                 JOIN spaces s ON s.id = m.space_id AND s.deleted_at IS NULL \
                 WHERE m.space_id = $1 AND m.user_id = $2)",
            )
            .bind(row.space_id)
            .bind(job.recipient_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
            if may_email {
                self.email.send_task_reminder(&row.email).await?;
            }
        }
        Ok(true)
    }

    pub async fn purge_stuck_claims(&self) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let updated = sqlx::query(
            "UPDATE task_reminder_jobs SET status = CASE WHEN attempts >= 8 THEN 'failed' ELSE 'pending' END, \
             available_at = now(), locked_at = NULL, last_error = 'worker lease expired' \
             WHERE status = 'sending' AND locked_at < $1 - interval '10 minutes'",
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(updated.rows_affected())
    }
}

#[derive(FromRow)]
struct ReminderJob {
    id: Uuid,
    task_id: Uuid,
    recipient_id: Uuid,
    offset_minutes: i32,
    attempts: i32,
}

#[derive(FromRow)]
struct ReminderCandidate {
    task_id: Uuid,
    recipient_id: Option<Uuid>,
    offsets_minutes: Option<Vec<i32>>,
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    time_zone: Option<String>,
    default_reminder_time: Option<NaiveTime>,
}

impl ReminderCandidate {
    fn scheduled_at(&self, offset_minutes: i32) -> Option<chrono::DateTime<Utc>> {
        reminder_time(
            self.due_date,
            self.due_time,
            self.time_zone.as_deref()?,
            self.default_reminder_time?,
            offset_minutes,
        )
    }
}

fn reminder_time(
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    time_zone: &str,
    default_reminder_time: NaiveTime,
    offset_minutes: i32,
) -> Option<chrono::DateTime<Utc>> {
    let time_zone = time_zone.parse::<chrono_tz::Tz>().ok()?;
    let (date, time, minutes) = match due_time {
        Some(time) => (due_date, time, offset_minutes),
        None => (
            due_date.checked_sub_days(Days::new((offset_minutes / 1440) as u64))?,
            default_reminder_time,
            offset_minutes % 1440,
        ),
    };
    resolve_local_time(time_zone, date.and_time(time))?
        .checked_sub_signed(Duration::minutes(i64::from(minutes)))
}

fn resolve_local_time(
    time_zone: chrono_tz::Tz,
    local_time: NaiveDateTime,
) -> Option<chrono::DateTime<Utc>> {
    use chrono::LocalResult;

    let mut candidate = local_time;
    for _ in 0..=180 {
        match time_zone.from_local_datetime(&candidate) {
            LocalResult::Single(value) => return Some(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, second) => {
                return Some(first.with_timezone(&Utc).min(second.with_timezone(&Utc)));
            }
            LocalResult::None => candidate = candidate.checked_add_signed(Duration::minutes(1))?,
        }
    }
    None
}

#[derive(FromRow)]
struct DeliveryRow {
    space_id: Uuid,
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    time_zone: String,
    default_reminder_time: NaiveTime,
    scheduled_at: chrono::DateTime<Utc>,
    title: String,
    email: String,
    in_app: bool,
    email_enabled: bool,
    muted: bool,
}

pub(crate) async fn push_reminder_is_current(
    pool: &PgPool,
    recipient: Uuid,
    task: Uuid,
    reference: chrono::DateTime<Utc>,
    offset: i32,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_as::<_, (NaiveDate, Option<NaiveTime>, String, NaiveTime)>(
        "SELECT t.due_date, t.due_time, u.time_zone, u.default_reminder_time \
         FROM tasks t JOIN users u ON u.id = COALESCE(t.assignee_id, t.creator_id) \
         LEFT JOIN task_reminder_preferences p ON p.task_id = t.id AND p.user_id = u.id \
         WHERE t.id = $1 AND u.id = $2 AND NOT t.done AND t.due_date IS NOT NULL \
           AND $3 = ANY(COALESCE(p.offsets_minutes, t.reminder_offsets_minutes)) \
           AND CASE WHEN t.due_time IS NULL THEN t.due_date >= (now() AT TIME ZONE u.time_zone)::date \
                    ELSE ((t.due_date + t.due_time) AT TIME ZONE u.time_zone) > now() END",
    )
    .bind(task)
    .bind(recipient)
    .bind(offset)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some_and(|(date, time, zone, default)| {
        reminder_time(date, time, &zone, default, offset) == Some(reference)
    }))
}

#[cfg(test)]
mod tests {
    use super::resolve_local_time;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    #[test]
    fn local_reminder_times_handle_dst_gaps_and_overlaps() {
        let new_york = chrono_tz::America::New_York;
        let gap = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            NaiveTime::from_hms_opt(2, 30, 0).unwrap(),
        );
        assert_eq!(
            resolve_local_time(new_york, gap).unwrap(),
            Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0).unwrap()
        );

        let overlap = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            NaiveTime::from_hms_opt(1, 30, 0).unwrap(),
        );
        assert_eq!(
            resolve_local_time(new_york, overlap).unwrap(),
            Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap()
        );
    }
}
