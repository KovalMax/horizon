use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateNotificationReadRequest {
    pub read: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationPreferenceInput {
    /// One of `task_assignment`, `membership_change`, `event_change`, `join_request`, `join_request_result`, or `due_reminder`.
    pub event_type: String,
    /// One of `in_app` or `email`.
    pub channel: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateNotificationPreferencesRequest {
    pub preferences: Vec<NotificationPreferenceInput>,
    pub muted_space_ids: Vec<Uuid>,
    pub default_reminder_time: String,
    pub time_zone: String,
}
