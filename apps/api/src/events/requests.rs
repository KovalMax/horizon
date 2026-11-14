use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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
    /// Inclusive last occurrence date in the series timezone. `None` means no end date.
    pub recurrence_ends_on: Option<String>,
}

#[derive(Clone, Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventRangeQuery {
    /// First local calendar date to include, inclusive.
    pub start_date: String,
    /// Last local calendar date to include, inclusive (maximum 90 days after start).
    pub end_date: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventListQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
