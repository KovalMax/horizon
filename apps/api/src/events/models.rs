use chrono::{DateTime, NaiveDate, Utc};
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
    pub recurrence_ends_on: Option<NaiveDate>,
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
    pub recurrence_ends_on: Option<NaiveDate>,
}

#[derive(Clone, Debug)]
pub(super) struct EventOccurrence {
    pub event: CalendarEvent,
    pub occurrence_date: NaiveDate,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub title: String,
    pub time_zone: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub is_override: bool,
}

#[derive(Clone, Debug)]
pub(super) struct OccurrenceOverride {
    pub event_id: Uuid,
    pub occurrence_date: NaiveDate,
    pub cancelled: bool,
    pub title: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub time_zone: Option<String>,
    pub all_day: Option<bool>,
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub enum EventError {
    Invalid,
    Forbidden,
    NotFound,
    Unavailable,
    InvalidOccurrence,
}
