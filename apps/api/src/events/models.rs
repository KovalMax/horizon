use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(super) struct CalendarEvent {
    pub id: Uuid,
    pub space_id: Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub time_zone: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub recurrence: Option<String>,
    pub space_name: String,
    pub space_kind: String,
}

#[derive(Clone, Debug)]
pub(super) struct EventDraft {
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub time_zone: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub recurrence: Option<String>,
}

#[derive(Debug)]
pub enum EventError {
    Invalid,
    Forbidden,
    NotFound,
    Unavailable,
}
