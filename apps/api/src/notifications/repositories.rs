use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::responses::{
    NotificationPageResponse, NotificationPreferenceResponse, NotificationResponse,
    NotificationSettingsResponse,
};

const EVENT_TYPES: [&str; 6] = [
    "task_assignment",
    "membership_change",
    "event_change",
    "join_request",
    "join_request_result",
    "due_reminder",
];
const CHANNELS: [&str; 2] = ["in_app", "email"];

#[derive(Clone)]
pub(super) struct PostgresNotificationsRepository {
    pool: PgPool,
}

impl PostgresNotificationsRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn list_for_user(
        &self,
        user_id: Uuid,
        window: PageWindow,
    ) -> Result<NotificationPageResponse, sqlx::Error> {
        let rows = sqlx::query_as::<_, NotificationRow>(
            "SELECT n.id, n.space_id, s.name AS space_name, n.event_type, n.title, n.body, \
                    n.destination, n.created_at, n.read_at \
             FROM notifications n JOIN spaces s ON s.id = n.space_id \
             WHERE n.recipient_id = $1 \
             ORDER BY n.created_at DESC, n.id DESC LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(window.limit + 1)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        let page: Page<NotificationResponse> =
            Page::from_rows(rows.into_iter().map(Into::into).collect(), window);
        let unread_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND read_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(NotificationPageResponse {
            items: page.items,
            next_offset: page.next_offset,
            unread_count,
        })
    }

    pub(super) async fn set_read(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
        read: bool,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE notifications SET read_at = CASE WHEN $3 THEN COALESCE(read_at, now()) ELSE NULL END \
             WHERE id = $1 AND recipient_id = $2",
        )
        .bind(notification_id)
        .bind(user_id)
        .bind(read)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(super) async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query(
            "UPDATE notifications SET read_at = now() WHERE recipient_id = $1 AND read_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    pub(super) async fn settings(
        &self,
        user_id: Uuid,
    ) -> Result<NotificationSettingsResponse, sqlx::Error> {
        let overrides = sqlx::query_as::<_, (String, String, bool)>(
            "SELECT event_type, channel, enabled FROM notification_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let muted_space_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT space_id FROM space_notification_preferences WHERE user_id = $1 AND muted ORDER BY space_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let (default_reminder_time, time_zone) = sqlx::query_as::<_, (chrono::NaiveTime, String)>(
            "SELECT default_reminder_time, time_zone FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let preferences = EVENT_TYPES
            .iter()
            .flat_map(|event_type| CHANNELS.iter().map(move |channel| (*event_type, *channel)))
            .map(|(event_type, channel)| {
                let default = channel == "in_app";
                let enabled = overrides
                    .iter()
                    .find(|(stored_type, stored_channel, _)| {
                        stored_type == event_type && stored_channel == channel
                    })
                    .map(|(_, _, enabled)| *enabled)
                    .unwrap_or(default);
                NotificationPreferenceResponse {
                    event_type: event_type.to_owned(),
                    channel: channel.to_owned(),
                    enabled,
                }
            })
            .collect();
        Ok(NotificationSettingsResponse {
            preferences,
            muted_space_ids,
            default_reminder_time: default_reminder_time.format("%H:%M").to_string(),
            time_zone,
        })
    }

    pub(super) async fn update_settings(
        &self,
        user_id: Uuid,
        preferences: &[(String, String, bool)],
        muted_space_ids: &[Uuid],
        default_reminder_time: &str,
        time_zone: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let member_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT space_id) FROM space_memberships \
             WHERE user_id = $1 AND space_id = ANY($2)",
        )
        .bind(user_id)
        .bind(muted_space_ids)
        .fetch_one(&mut *transaction)
        .await?;
        if member_count
            != muted_space_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len() as i64
        {
            return Ok(false);
        }
        sqlx::query("DELETE FROM notification_preferences WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        for (event_type, channel, enabled) in preferences {
            sqlx::query(
                "INSERT INTO notification_preferences (user_id, event_type, channel, enabled) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(user_id)
            .bind(event_type)
            .bind(channel)
            .bind(enabled)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query("UPDATE users SET default_reminder_time = $2, time_zone = $3 WHERE id = $1")
            .bind(user_id)
            .bind(
                chrono::NaiveTime::parse_from_str(default_reminder_time, "%H:%M")
                    .map_err(|_| sqlx::Error::Protocol("invalid reminder time".into()))?,
            )
            .bind(time_zone)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM space_notification_preferences WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        for space_id in muted_space_ids {
            sqlx::query(
                "INSERT INTO space_notification_preferences (user_id, space_id, muted) VALUES ($1, $2, TRUE)",
            )
            .bind(user_id)
            .bind(space_id)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query(
            "UPDATE task_reminder_jobs j SET status = 'cancelled', locked_at = NULL \
             FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
             JOIN space_notification_preferences p ON p.space_id = l.space_id \
             WHERE j.task_id = t.id AND j.recipient_id = p.user_id AND p.user_id = $1 \
               AND p.muted AND j.status = 'pending'",
        )
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub(super) async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        Ok(
            sqlx::query("DELETE FROM notifications WHERE created_at <= $1 - interval '90 days'")
                .bind(now)
                .execute(&self.pool)
                .await?
                .rows_affected(),
        )
    }
}

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    space_id: Uuid,
    space_name: String,
    event_type: String,
    title: String,
    body: String,
    destination: String,
    created_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
}

impl From<NotificationRow> for NotificationResponse {
    fn from(row: NotificationRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            space_name: row.space_name,
            event_type: row.event_type,
            title: row.title,
            body: row.body,
            destination: row.destination,
            created_at: row.created_at.to_rfc3339(),
            read_at: row.read_at.map(|value| value.to_rfc3339()),
        }
    }
}
