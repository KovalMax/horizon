use chrono::{NaiveDate, NaiveTime};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::{Page, PageWindow};

use super::{
    models::{TaskDraft, TodoError},
    repositories::PostgresTodosRepository,
    requests::{
        CreateTaskListRequest, CreateTaskRequest, UpdateTaskListRequest, UpdateTaskRequest,
    },
    responses::{TaskBoardResponse, TaskListResponse, TaskResponse},
};

#[derive(Clone)]
pub struct TodosApplication {
    repository: PostgresTodosRepository,
}

impl TodosApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresTodosRepository::new(pool),
        }
    }

    pub async fn list_for_member(
        &self,
        user_id: Uuid,
        lists_page: PageWindow,
        tasks_page: PageWindow,
    ) -> Result<TaskBoardResponse, TodoError> {
        let (lists, tasks) = self
            .repository
            .list_for_member(user_id, lists_page, tasks_page)
            .await
            .map_err(|_| TodoError::Unavailable)?;
        let lists = Page::from_rows(lists, lists_page);
        let tasks = Page::from_rows(tasks, tasks_page);
        Ok(TaskBoardResponse {
            lists: lists.items.into_iter().map(Into::into).collect(),
            tasks: tasks.items.into_iter().map(Into::into).collect(),
            lists_next_offset: lists.next_offset,
            tasks_next_offset: tasks.next_offset,
        })
    }

    pub async fn create_list(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        request: &CreateTaskListRequest,
    ) -> Result<TaskListResponse, TodoError> {
        let name = validate_list_name(&request.name)?;
        self.repository
            .create_list(space_id, user_id, name)
            .await
            .map(Into::into)
    }

    pub async fn update_list(
        &self,
        list_id: Uuid,
        user_id: Uuid,
        request: &UpdateTaskListRequest,
    ) -> Result<TaskListResponse, TodoError> {
        let name = validate_list_name(&request.name)?;
        self.repository
            .update_list(list_id, user_id, name)
            .await
            .map(Into::into)
    }

    pub async fn delete_list(&self, list_id: Uuid, user_id: Uuid) -> Result<(), TodoError> {
        self.repository.delete_list(list_id, user_id).await
    }

    pub async fn create_task(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        request: &CreateTaskRequest,
    ) -> Result<TaskResponse, TodoError> {
        let draft = validate(request)?;
        self.repository
            .create_task(space_id, user_id, &draft)
            .await
            .map(Into::into)
    }

    pub async fn update_completion(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        done: bool,
    ) -> Result<TaskResponse, TodoError> {
        self.repository
            .update_completion(task_id, user_id, done)
            .await
            .map(Into::into)
    }

    pub async fn update_task(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        request: &UpdateTaskRequest,
    ) -> Result<TaskResponse, TodoError> {
        let draft = validate_task_fields(
            &request.title,
            &request.list_name,
            request.due_date.as_deref(),
            request.due_time.as_deref(),
            request.assignee_id,
            &request.reminder_offsets_minutes,
        )?;
        self.repository
            .update_task(task_id, user_id, &draft)
            .await
            .map(Into::into)
    }

    pub async fn delete_task(&self, task_id: Uuid, user_id: Uuid) -> Result<(), TodoError> {
        self.repository.delete_task(task_id, user_id).await
    }

    pub async fn set_my_reminder_preference(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        offsets: &[i32],
    ) -> Result<(), TodoError> {
        if offsets.iter().any(|offset| ![1440, 60].contains(offset))
            || offsets
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != offsets.len()
        {
            return Err(TodoError::Invalid);
        }
        if self
            .repository
            .set_my_reminder_preference(task_id, user_id, offsets)
            .await
            .map_err(|_| TodoError::Unavailable)?
        {
            Ok(())
        } else {
            Err(TodoError::NotFound)
        }
    }
}

fn validate(request: &CreateTaskRequest) -> Result<TaskDraft, TodoError> {
    validate_task_fields(
        &request.title,
        &request.list_name,
        request.due_date.as_deref(),
        request.due_time.as_deref(),
        request.assignee_id,
        &request.reminder_offsets_minutes,
    )
}

fn validate_list_name(name: &str) -> Result<&str, TodoError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(TodoError::Invalid);
    }
    Ok(name)
}

fn validate_task_fields(
    title: &str,
    list_name: &str,
    due_date: Option<&str>,
    due_time: Option<&str>,
    assignee_id: Option<Uuid>,
    reminder_offsets_minutes: &[i32],
) -> Result<TaskDraft, TodoError> {
    let title = title.trim();
    let list_name = list_name.trim();
    if title.is_empty()
        || title.chars().count() > 160
        || list_name.is_empty()
        || list_name.chars().count() > 80
    {
        return Err(TodoError::Invalid);
    }
    let due_date = due_date
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| TodoError::Invalid))
        .transpose()?;
    let due_time = due_time
        .map(|value| {
            NaiveTime::parse_from_str(value, "%H:%M")
                .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
                .map_err(|_| TodoError::Invalid)
        })
        .transpose()?;
    if reminder_offsets_minutes
        .iter()
        .any(|offset| ![1440, 60].contains(offset))
    {
        return Err(TodoError::Invalid);
    }
    let mut reminder_offsets_minutes = reminder_offsets_minutes.to_vec();
    reminder_offsets_minutes.sort_unstable();
    reminder_offsets_minutes.dedup();
    Ok(TaskDraft {
        title: title.to_owned(),
        list_name: list_name.to_owned(),
        due_date,
        due_time,
        reminder_offsets_minutes,
        assignee_id,
    })
}
