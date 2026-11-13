use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateEventRequest {
    pub title: String,
    /// RFC 3339 timestamp with an explicit offset.
    pub starts_at: String,
    /// RFC 3339 timestamp with an explicit offset; must be after `starts_at`.
    pub ends_at: Option<String>,
    /// IANA timezone name, for example `Europe/Berlin`.
    pub time_zone: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    /// One of `daily`, `weekly`, or `monthly`.
    pub recurrence: Option<String>,
}
