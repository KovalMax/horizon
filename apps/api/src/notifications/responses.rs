use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub destination: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationPageResponse {
    pub items: Vec<NotificationResponse>,
    pub next_offset: Option<i64>,
    pub unread_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationPreferenceResponse {
    pub event_type: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NotificationSettingsResponse {
    pub preferences: Vec<NotificationPreferenceResponse>,
    pub muted_space_ids: Vec<Uuid>,
    pub default_reminder_time: String,
    pub time_zone: String,
}
