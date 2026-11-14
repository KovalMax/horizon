use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::PageWindow;

use super::{
    repositories::PostgresNotificationsRepository,
    requests::NotificationPreferenceInput,
    responses::{NotificationPageResponse, NotificationSettingsResponse},
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
pub struct NotificationsApplication {
    repository: PostgresNotificationsRepository,
}

impl NotificationsApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresNotificationsRepository::new(pool),
        }
    }

    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        page: PageWindow,
    ) -> Result<NotificationPageResponse, NotificationError> {
        self.repository
            .list_for_user(user_id, page)
            .await
            .map_err(|_| NotificationError::Unavailable)
    }

    pub async fn set_read(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
        read: bool,
    ) -> Result<(), NotificationError> {
        if self
            .repository
            .set_read(user_id, notification_id, read)
            .await
            .map_err(|_| NotificationError::Unavailable)?
        {
            Ok(())
        } else {
            Err(NotificationError::NotFound)
        }
    }

    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64, NotificationError> {
        self.repository
            .mark_all_read(user_id)
            .await
            .map_err(|_| NotificationError::Unavailable)
    }

    pub async fn settings(
        &self,
        user_id: Uuid,
    ) -> Result<NotificationSettingsResponse, NotificationError> {
        self.repository
            .settings(user_id)
            .await
            .map_err(|_| NotificationError::Unavailable)
    }

    pub async fn update_settings(
        &self,
        user_id: Uuid,
        preferences: &[NotificationPreferenceInput],
        muted_space_ids: &[Uuid],
        default_reminder_time: &str,
        time_zone: &str,
    ) -> Result<NotificationSettingsResponse, NotificationError> {
        let expected = EVENT_TYPES.len() * CHANNELS.len();
        if preferences.len() != expected
            || preferences.iter().any(|preference| {
                !EVENT_TYPES.contains(&preference.event_type.as_str())
                    || !CHANNELS.contains(&preference.channel.as_str())
            })
            || preferences
                .iter()
                .map(|preference| (&preference.event_type, &preference.channel))
                .collect::<std::collections::HashSet<_>>()
                .len()
                != expected
            || muted_space_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != muted_space_ids.len()
        {
            return Err(NotificationError::InvalidPreferences);
        }
        if chrono::NaiveTime::parse_from_str(default_reminder_time, "%H:%M").is_err()
            || time_zone.parse::<chrono_tz::Tz>().is_err()
        {
            return Err(NotificationError::InvalidPreferences);
        }
        let preferences = preferences
            .iter()
            .map(|preference| {
                (
                    preference.event_type.clone(),
                    preference.channel.clone(),
                    preference.enabled,
                )
            })
            .collect::<Vec<_>>();
        match self
            .repository
            .update_settings(
                user_id,
                &preferences,
                muted_space_ids,
                default_reminder_time,
                time_zone,
            )
            .await
            .map_err(|_| NotificationError::Unavailable)?
        {
            true => self.settings(user_id).await,
            false => Err(NotificationError::InvalidPreferences),
        }
    }

    pub async fn purge_expired(&self) -> Result<u64, sqlx::Error> {
        self.repository.purge_expired(Utc::now()).await
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationError {
    InvalidPreferences,
    NotFound,
    Unavailable,
}
