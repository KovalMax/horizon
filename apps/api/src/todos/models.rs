use chrono::{NaiveDate, NaiveTime};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(super) struct TaskList {
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub space_name: String,
    pub space_kind: String,
}

#[derive(Clone, Debug)]
pub(super) struct Task {
    pub id: Uuid,
    pub space_id: Uuid,
    pub list_id: Uuid,
    pub title: String,
    pub list_name: String,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub done: bool,
    pub assignee_id: Option<Uuid>,
    pub assignee_email: Option<String>,
    pub space_name: String,
    pub space_kind: String,
}

#[derive(Clone, Debug)]
pub(super) struct TaskDraft {
    pub title: String,
    pub list_name: String,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub assignee_id: Option<Uuid>,
}

#[derive(Debug)]
pub enum TodoError {
    Invalid,
    Conflict,
    Forbidden,
    NotFound,
    Unavailable,
}
