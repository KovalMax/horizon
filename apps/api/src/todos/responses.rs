use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::{Task, TaskList};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskListResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    /// `personal` or `shared`.
    pub space_kind: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub space_name: String,
    /// `personal` or `shared`.
    pub space_kind: String,
    pub list_id: Uuid,
    pub list_name: String,
    pub title: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub done: bool,
    pub assignee_id: Option<Uuid>,
    pub assignee_email: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskBoardResponse {
    pub lists: Vec<TaskListResponse>,
    pub tasks: Vec<TaskResponse>,
    pub lists_next_offset: Option<i64>,
    pub tasks_next_offset: Option<i64>,
}

impl From<TaskList> for TaskListResponse {
    fn from(list: TaskList) -> Self {
        Self {
            id: list.id,
            space_id: list.space_id,
            space_name: list.space_name,
            space_kind: list.space_kind,
            name: list.name,
        }
    }
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            space_id: task.space_id,
            space_name: task.space_name,
            space_kind: task.space_kind,
            list_id: task.list_id,
            list_name: task.list_name,
            title: task.title,
            due_date: task
                .due_date
                .map(|date| date.format("%Y-%m-%d").to_string()),
            due_time: task.due_time.map(|time| time.format("%H:%M").to_string()),
            done: task.done,
            assignee_id: task.assignee_id,
            assignee_email: task.assignee_email,
        }
    }
}
