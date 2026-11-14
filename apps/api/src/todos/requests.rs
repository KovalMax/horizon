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
    /// Reminder offsets in minutes before the due time: 1440 (1 day) and/or 60 (1 hour).
    #[serde(default = "default_reminder_offsets")]
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
    /// Reminder offsets in minutes before the due time: 1440 (1 day) and/or 60 (1 hour).
    #[serde(default = "default_reminder_offsets")]
    pub reminder_offsets_minutes: Vec<i32>,
}

fn default_reminder_offsets() -> Vec<i32> {
    vec![1440, 60]
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskCompletionRequest {
    pub done: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTaskReminderPreferenceRequest {
    /// Private offsets for the authenticated Task recipient: 1440 and/or 60.
    pub offsets_minutes: Vec<i32>,
}
