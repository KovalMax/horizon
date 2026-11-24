use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::{CalendarEvent, EventOccurrence};

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
    pub recurrence_ends_on: Option<String>,
    /// Local scheduled date for an expanded occurrence; absent on series CRUD responses.
    pub occurrence_date: Option<String>,
    /// Stable composite occurrence identity (`series UUID:scheduled local date`).
    pub occurrence_id: Option<String>,
    pub is_override: bool,
    /// Reminder offsets visible only to the authenticated Member who set them.
    pub my_reminder_offsets_minutes: Vec<i32>,
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
            recurrence_ends_on: event.recurrence_ends_on.map(|date| date.to_string()),
            occurrence_date: None,
            occurrence_id: None,
            is_override: false,
            my_reminder_offsets_minutes: event.my_reminder_offsets_minutes,
        }
    }
}

impl From<EventOccurrence> for EventResponse {
    fn from(occurrence: EventOccurrence) -> Self {
        let event_id = occurrence.event.id;
        let mut response = EventResponse::from(occurrence.event);
        response.title = occurrence.title;
        response.starts_at = occurrence.starts_at.to_rfc3339();
        response.ends_at = occurrence.ends_at.map(|value| value.to_rfc3339());
        response.time_zone = occurrence.time_zone;
        response.all_day = occurrence.all_day;
        response.location = occurrence.location;
        response.notes = occurrence.notes;
        response.occurrence_date = Some(occurrence.occurrence_date.to_string());
        response.occurrence_id = Some(format!("{}:{}", event_id, occurrence.occurrence_date));
        response.is_override = occurrence.is_override;
        response
    }
}
