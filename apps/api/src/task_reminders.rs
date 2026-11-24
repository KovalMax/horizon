use std::sync::Arc;

use chrono::{Days, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use sqlx::{postgres::PgPool, FromRow};
use uuid::Uuid;

use crate::auth::EmailSender;

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
        self.reconcile(Utc::now()).await?;
        while let Some(job) = self.claim(now).await? {
            self.deliver(job).await;
        }
        Ok(())
    }

    async fn reconcile(&self, now: chrono::DateTime<Utc>) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE task_reminder_jobs j SET status = 'cancelled', locked_at = NULL \
             WHERE j.status = 'pending' AND NOT EXISTS ( \
               SELECT 1 FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
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
                      WHERE user_id = j.recipient_id AND event_type = 'due_reminder' AND channel = 'email'), FALSE)) \
                 AND j.offset_minutes = ANY(COALESCE(p.offsets_minutes, t.reminder_offsets_minutes)) \
             )",
        )
        .execute(&mut *tx)
        .await?;
        let candidates = sqlx::query_as::<_, ReminderCandidate>(
            "SELECT t.id AS task_id, u.id AS recipient_id, \
               COALESCE(p.offsets_minutes, t.reminder_offsets_minutes) AS offsets_minutes, \
               t.due_date, t.due_time, u.time_zone, u.default_reminder_time \
             FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
             JOIN users u ON u.id = COALESCE(t.assignee_id, t.creator_id) \
             LEFT JOIN task_reminder_preferences p ON p.task_id = t.id AND p.user_id = u.id \
             WHERE NOT t.done AND t.due_date IS NOT NULL \
               AND NOT EXISTS (SELECT 1 FROM space_notification_preferences mute \
                 WHERE mute.user_id = u.id AND mute.space_id = l.space_id AND mute.muted) \
               AND (COALESCE((SELECT enabled FROM notification_preferences \
                    WHERE user_id = u.id AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) \
                 OR COALESCE((SELECT enabled FROM notification_preferences \
                    WHERE user_id = u.id AND event_type = 'due_reminder' AND channel = 'email'), FALSE))",
        )
        .fetch_all(&mut *tx)
        .await?;
        for candidate in candidates {
            for offset_minutes in &candidate.offsets_minutes {
                let Some(scheduled_at) = candidate.scheduled_at(*offset_minutes) else {
                    continue;
                };
                if scheduled_at <= now {
                    continue;
                }
                sqlx::query(
                    "INSERT INTO task_reminder_jobs \
                     (id, task_id, recipient_id, offset_minutes, scheduled_at, available_at) \
                     VALUES (gen_random_uuid(), $1, $2, $3, $4, $4) \
                     ON CONFLICT (task_id, recipient_id, offset_minutes) DO UPDATE \
                       SET status = 'pending', sent_at = NULL, last_error = NULL, \
                           scheduled_at = EXCLUDED.scheduled_at, \
                           available_at = CASE WHEN task_reminder_jobs.scheduled_at IS DISTINCT FROM EXCLUDED.scheduled_at \
                             THEN EXCLUDED.available_at ELSE task_reminder_jobs.available_at END \
                       WHERE task_reminder_jobs.status IN ('pending', 'cancelled') \
                         AND EXCLUDED.scheduled_at > $5",
                )
                .bind(candidate.task_id)
                .bind(candidate.recipient_id)
                .bind(*offset_minutes)
                .bind(scheduled_at)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await
    }

    async fn claim(&self, now: chrono::DateTime<Utc>) -> Result<Option<ReminderJob>, sqlx::Error> {
        sqlx::query_as::<_, ReminderJob>(
            "UPDATE task_reminder_jobs SET status = 'sending', locked_at = now(), attempts = attempts + 1 \
             WHERE id = (SELECT id FROM task_reminder_jobs \
               WHERE status = 'pending' AND scheduled_at <= $1 AND available_at <= $1 \
               ORDER BY scheduled_at, id FOR UPDATE SKIP LOCKED LIMIT 1) \
             RETURNING id, task_id, recipient_id, offset_minutes, attempts",
        )
        .bind(now)
        .fetch_optional(&self.pool)
        .await
    }

    async fn deliver(&self, job: ReminderJob) {
        let result = self.deliver_claimed(&job).await;
        match result {
            Ok(()) => {
                let _ = sqlx::query(
                    "UPDATE task_reminder_jobs SET status = 'sent', sent_at = now(), locked_at = NULL, last_error = NULL WHERE id = $1",
                )
                .bind(job.id)
                .execute(&self.pool)
                .await;
            }
            Err(error) => {
                let terminal = job.attempts >= 8;
                let delay_seconds =
                    60_i64.saturating_mul(1_i64 << (job.attempts.saturating_sub(1).min(10)));
                let _ = sqlx::query(
                    "UPDATE task_reminder_jobs SET status = $2, locked_at = NULL, \
                     available_at = now() + ($3 * interval '1 second'), last_error = $4 WHERE id = $1",
                )
                .bind(job.id)
                .bind(if terminal { "failed" } else { "pending" })
                .bind(delay_seconds)
                .bind(error)
                .execute(&self.pool)
                .await;
            }
        }
    }

    async fn deliver_claimed(&self, job: &ReminderJob) -> Result<(), String> {
        let row = sqlx::query_as::<_, DeliveryRow>(
            "SELECT t.title, s.name AS space_name, t.due_date, t.due_time, \
               u.time_zone, u.default_reminder_time, j.scheduled_at, \
               to_char(t.due_date, 'YYYY-MM-DD') || COALESCE(' ' || to_char(t.due_time, 'HH24:MI'), '') AS due, \
               e.email_normalized AS email, \
               COALESCE((SELECT enabled FROM notification_preferences WHERE user_id = j.recipient_id \
                  AND event_type = 'due_reminder' AND channel = 'in_app'), TRUE) AS in_app, \
               COALESCE((SELECT enabled FROM notification_preferences WHERE user_id = j.recipient_id \
                  AND event_type = 'due_reminder' AND channel = 'email'), FALSE) AS email_enabled, \
               EXISTS (SELECT 1 FROM space_notification_preferences WHERE user_id = j.recipient_id \
                  AND space_id = l.space_id AND muted) AS muted \
             FROM task_reminder_jobs j JOIN tasks t ON t.id = j.task_id \
             JOIN todo_lists l ON l.id = t.list_id JOIN spaces s ON s.id = l.space_id \
             JOIN users u ON u.id = j.recipient_id \
             JOIN LATERAL (SELECT email_normalized FROM email_identities \
               WHERE user_id = j.recipient_id ORDER BY created_at LIMIT 1) e ON TRUE \
             WHERE j.id = $1 AND NOT t.done AND t.due_date IS NOT NULL \
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
                && (row.in_app || row.email_enabled)
                && reminder_time(
                    row.due_date,
                    row.due_time,
                    &row.time_zone,
                    row.default_reminder_time,
                    job.offset_minutes,
                ) == Some(row.scheduled_at)
        }) else {
            return Ok(());
        };

        if row.in_app {
            let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
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
            tx.commit().await.map_err(|error| error.to_string())?;
        }
        if row.email_enabled {
            self.email
                .send_task_reminder(&row.email, &row.title, &row.space_name, &row.due)
                .await?;
        }
        Ok(())
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
    recipient_id: Uuid,
    offsets_minutes: Vec<i32>,
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    time_zone: String,
    default_reminder_time: NaiveTime,
}

impl ReminderCandidate {
    fn scheduled_at(&self, offset_minutes: i32) -> Option<chrono::DateTime<Utc>> {
        reminder_time(
            self.due_date,
            self.due_time,
            &self.time_zone,
            self.default_reminder_time,
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
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    time_zone: String,
    default_reminder_time: NaiveTime,
    scheduled_at: chrono::DateTime<Utc>,
    title: String,
    space_name: String,
    due: String,
    email: String,
    in_app: bool,
    email_enabled: bool,
    muted: bool,
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
