use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateTaskListRequest {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskListRequest {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateTaskRequest {
    pub title: String,
    pub list_name: String,
    /// ISO date (`YYYY-MM-DD`).
    pub due_date: Option<String>,
    /// 24-hour time (`HH:MM`).
    pub due_time: Option<String>,
    /// ID of a current Member of the Task's Space; omit or set null to leave unassigned.
    pub assignee_id: Option<uuid::Uuid>,
    /// Up to five distinct offsets in minutes: 15, 30, 60, 120, 1440, 2880, 10080. Defaults to none.
    #[serde(default)]
    pub reminder_offsets_minutes: Vec<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskRequest {
    pub title: String,
    pub list_name: String,
    /// ISO date (`YYYY-MM-DD`).
    pub due_date: Option<String>,
    /// 24-hour time (`HH:MM`).
    pub due_time: Option<String>,
    /// ID of a current Member of the Task's Space; omit or set null to leave unassigned.
    pub assignee_id: Option<uuid::Uuid>,
    /// Up to five distinct offsets in minutes: 15, 30, 60, 120, 1440, 2880, 10080.
    #[serde(default)]
    pub reminder_offsets_minutes: Vec<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskCompletionRequest {
    pub done: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskReminderPreferenceRequest {
    /// Private offsets for the authenticated Task recipient, using the same presets as the Task.
    pub offsets_minutes: Vec<i32>,
}
