use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::CalendarEvent;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    /// `personal` or `shared`.
    pub space_kind: String,
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub time_zone: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub recurrence: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EventPageResponse {
    pub items: Vec<EventResponse>,
    pub next_offset: Option<i64>,
}

impl From<CalendarEvent> for EventResponse {
    fn from(event: CalendarEvent) -> Self {
        Self {
            id: event.id,
            space_id: event.space_id,
            space_name: event.space_name,
            space_kind: event.space_kind,
            title: event.title,
            starts_at: event.starts_at.to_rfc3339(),
            ends_at: event.ends_at.map(|value| value.to_rfc3339()),
            time_zone: event.time_zone,
            all_day: event.all_day,
            location: event.location,
            notes: event.notes,
            recurrence: event.recurrence,
        }
    }
}
