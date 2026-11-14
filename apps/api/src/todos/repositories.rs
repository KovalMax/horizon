use chrono::{NaiveDate, NaiveTime};
use sqlx::PgPool;
use uuid::Uuid;

use crate::pagination::PageWindow;

use super::models::{Task, TaskDraft, TaskList, TodoError};

#[derive(Clone)]
pub(super) struct PostgresTodosRepository {
    pool: PgPool,
}

impl PostgresTodosRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn set_my_reminder_preference(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        offsets: &[i32],
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO task_reminder_preferences (task_id, user_id, offsets_minutes) \
             SELECT t.id, $2, $3 FROM tasks t \
             WHERE t.id = $1 AND COALESCE(t.assignee_id, t.creator_id) = $2 \
             ON CONFLICT (task_id, user_id) DO UPDATE SET offsets_minutes = EXCLUDED.offsets_minutes, updated_at = now()",
        )
        .bind(task_id)
        .bind(user_id)
        .bind(offsets)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(super) async fn list_for_member(
        &self,
        user_id: Uuid,
        lists_page: PageWindow,
        tasks_page: PageWindow,
    ) -> Result<(Vec<TaskList>, Vec<Task>), sqlx::Error> {
        let lists = sqlx::query_as::<_, TaskListRow>(
            "SELECT l.id, l.space_id, l.name, s.name AS space_name, s.kind AS space_kind \
             FROM todo_lists l \
             JOIN spaces s ON s.id = l.space_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $1 \
             ORDER BY l.created_at, l.name, l.id LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(lists_page.limit + 1)
        .bind(lists_page.offset)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
        let tasks = sqlx::query_as::<_, TaskRow>(
            "SELECT t.id, l.space_id, l.id AS list_id, t.title, l.name AS list_name, \
             t.due_date, t.due_time, t.reminder_offsets_minutes, \
             COALESCE((SELECT offsets_minutes FROM task_reminder_preferences WHERE task_id = t.id AND user_id = $1), t.reminder_offsets_minutes) AS my_reminder_offsets_minutes, \
             t.done, t.assignee_id, assigned.email_normalized AS assignee_email, \
             s.name AS space_name, s.kind AS space_kind \
             FROM tasks t \
             JOIN todo_lists l ON l.id = t.list_id \
             JOIN spaces s ON s.id = l.space_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $1 \
             LEFT JOIN LATERAL ( \
               SELECT email_normalized FROM email_identities \
               WHERE user_id = t.assignee_id ORDER BY created_at LIMIT 1 \
             ) assigned ON TRUE \
             ORDER BY t.due_date NULLS LAST, t.due_time NULLS LAST, t.created_at, t.id \
             LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(tasks_page.limit + 1)
        .bind(tasks_page.offset)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
        Ok((lists, tasks))
    }

    pub(super) async fn create_list(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        name: &str,
    ) -> Result<TaskList, TodoError> {
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO todo_lists (id, space_id, name) \
             SELECT $1, $2, $3 \
             WHERE EXISTS (SELECT 1 FROM space_memberships \
                           WHERE space_id = $2 AND user_id = $4 AND role IN ('owner', 'editor')) \
             ON CONFLICT (space_id, name) DO NOTHING \
             RETURNING id",
        )
        .bind(id)
        .bind(space_id)
        .bind(name)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;

        if let Some(id) = inserted {
            return self
                .find_list(id, user_id)
                .await?
                .ok_or(TodoError::NotFound);
        }
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT l.id FROM todo_lists l \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             WHERE l.space_id = $1 AND l.name = $3",
        )
        .bind(space_id)
        .bind(user_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if let Some(id) = existing {
            return self
                .find_list(id, user_id)
                .await?
                .ok_or(TodoError::NotFound);
        }
        Err(self.space_write_error(space_id, user_id).await?)
    }

    pub(super) async fn update_list(
        &self,
        list_id: Uuid,
        user_id: Uuid,
        name: &str,
    ) -> Result<TaskList, TodoError> {
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE todo_lists l SET name = $3 \
             WHERE l.id = $1 AND EXISTS ( \
               SELECT 1 FROM space_memberships m \
               WHERE m.space_id = l.space_id AND m.user_id = $2 \
                 AND m.role IN ('owner', 'editor') \
             ) RETURNING l.id",
        )
        .bind(list_id)
        .bind(user_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|db| db.code().as_deref() == Some("23505"))
            {
                TodoError::Conflict
            } else {
                TodoError::Unavailable
            }
        })?;
        if updated.is_none() {
            return Err(self.list_write_error(list_id, user_id).await?);
        }
        self.find_list(list_id, user_id)
            .await?
            .ok_or(TodoError::NotFound)
    }

    pub(super) async fn delete_list(&self, list_id: Uuid, user_id: Uuid) -> Result<(), TodoError> {
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM todo_lists l USING space_memberships m \
             WHERE l.id = $1 AND m.space_id = l.space_id AND m.user_id = $2 \
               AND m.role IN ('owner', 'editor') RETURNING l.id",
        )
        .bind(list_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if deleted.is_none() {
            return Err(self.list_write_error(list_id, user_id).await?);
        }
        Ok(())
    }

    pub(super) async fn create_task(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        draft: &TaskDraft,
    ) -> Result<Task, TodoError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| TodoError::Unavailable)?;
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        match role.as_deref() {
            Some("owner" | "editor") => {}
            Some("viewer") => return Err(TodoError::Forbidden),
            _ => return Err(TodoError::NotFound),
        }

        if let Some(assignee_id) = draft.assignee_id {
            let member = sqlx::query_scalar::<_, Uuid>(
                "SELECT user_id FROM space_memberships \
                 WHERE space_id = $1 AND user_id = $2 FOR KEY SHARE",
            )
            .bind(space_id)
            .bind(assignee_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| TodoError::Unavailable)?;
            if member.is_none() {
                return Err(TodoError::Invalid);
            }
        }

        let list_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO todo_lists (id, space_id, name) VALUES ($1, $2, $3) \
             ON CONFLICT (space_id, name) DO UPDATE SET name = EXCLUDED.name \
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(space_id)
        .bind(&draft.list_name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, list_id, creator_id, title, due_date, due_time, assignee_id, reminder_offsets_minutes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(task_id)
        .bind(list_id)
        .bind(user_id)
        .bind(&draft.title)
        .bind(draft.due_date)
        .bind(draft.due_time)
        .bind(draft.assignee_id)
        .bind(&draft.reminder_offsets_minutes)
        .execute(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if let Some(assignee_id) = draft.assignee_id {
            crate::notifications::enqueue_notification(
                &mut transaction,
                space_id,
                user_id,
                assignee_id,
                &crate::notifications::NotificationContent {
                    event_type: "task_assignment",
                    title: "Task assigned to you",
                    body: &draft.title,
                    destination: "tasks",
                },
            )
            .await
            .map_err(|_| TodoError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| TodoError::Unavailable)?;
        self.find_task(task_id, user_id)
            .await?
            .ok_or(TodoError::NotFound)
    }

    pub(super) async fn update_completion(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        done: bool,
    ) -> Result<Task, TodoError> {
        let updated = sqlx::query_scalar::<_, Uuid>(
            "UPDATE tasks t SET done = $3, updated_at = now() \
             WHERE t.id = $1 AND EXISTS ( \
               SELECT 1 FROM todo_lists l \
               JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
               WHERE l.id = t.list_id AND m.role IN ('owner', 'editor') \
             ) RETURNING t.id",
        )
        .bind(task_id)
        .bind(user_id)
        .bind(done)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if updated.is_none() {
            return Err(self.task_write_error(task_id, user_id).await?);
        }
        self.find_task(task_id, user_id)
            .await?
            .ok_or(TodoError::NotFound)
    }

    pub(super) async fn update_task(
        &self,
        task_id: Uuid,
        user_id: Uuid,
        draft: &TaskDraft,
    ) -> Result<Task, TodoError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| TodoError::Unavailable)?;
        let task_context = sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
            "SELECT l.space_id, m.role, t.assignee_id FROM tasks t \
             JOIN todo_lists l ON l.id = t.list_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             WHERE t.id = $1 FOR UPDATE OF t",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        let Some((space_id, role, previous_assignee_id)) = task_context else {
            return Err(self.task_write_error(task_id, user_id).await?);
        };
        if role == "viewer" {
            return Err(TodoError::Forbidden);
        }

        if let Some(assignee_id) = draft.assignee_id {
            let member = sqlx::query_scalar::<_, Uuid>(
                "SELECT user_id FROM space_memberships \
                 WHERE space_id = $1 AND user_id = $2 FOR KEY SHARE",
            )
            .bind(space_id)
            .bind(assignee_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| TodoError::Unavailable)?;
            if member.is_none() {
                return Err(TodoError::Invalid);
            }
        }

        let list_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO todo_lists (id, space_id, name) VALUES ($1, $2, $3) \
             ON CONFLICT (space_id, name) DO UPDATE SET name = EXCLUDED.name \
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(space_id)
        .bind(&draft.list_name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        sqlx::query(
            "UPDATE tasks SET title = $2, list_id = $3, due_date = $4, due_time = $5, \
             assignee_id = $6, reminder_offsets_minutes = $7, updated_at = now() WHERE id = $1",
        )
        .bind(task_id)
        .bind(&draft.title)
        .bind(list_id)
        .bind(draft.due_date)
        .bind(draft.due_time)
        .bind(draft.assignee_id)
        .bind(&draft.reminder_offsets_minutes)
        .execute(&mut *transaction)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if let Some(assignee_id) = draft
            .assignee_id
            .filter(|id| Some(*id) != previous_assignee_id)
        {
            crate::notifications::enqueue_notification(
                &mut transaction,
                space_id,
                user_id,
                assignee_id,
                &crate::notifications::NotificationContent {
                    event_type: "task_assignment",
                    title: "Task assigned to you",
                    body: &draft.title,
                    destination: "tasks",
                },
            )
            .await
            .map_err(|_| TodoError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| TodoError::Unavailable)?;
        self.find_task(task_id, user_id)
            .await?
            .ok_or(TodoError::NotFound)
    }

    pub(super) async fn delete_task(&self, task_id: Uuid, user_id: Uuid) -> Result<(), TodoError> {
        let deleted = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM tasks t USING todo_lists l, space_memberships m \
             WHERE t.id = $1 AND l.id = t.list_id \
               AND m.space_id = l.space_id AND m.user_id = $2 \
               AND m.role IN ('owner', 'editor') RETURNING t.id",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        if deleted.is_none() {
            return Err(self.task_write_error(task_id, user_id).await?);
        }
        Ok(())
    }

    async fn find_list(&self, list_id: Uuid, user_id: Uuid) -> Result<Option<TaskList>, TodoError> {
        sqlx::query_as::<_, TaskListRow>(
            "SELECT l.id, l.space_id, l.name, s.name AS space_name, s.kind AS space_kind \
             FROM todo_lists l JOIN spaces s ON s.id = l.space_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             WHERE l.id = $1",
        )
        .bind(list_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(|_| TodoError::Unavailable)
    }

    async fn find_task(&self, task_id: Uuid, user_id: Uuid) -> Result<Option<Task>, TodoError> {
        sqlx::query_as::<_, TaskRow>(
            "SELECT t.id, l.space_id, l.id AS list_id, t.title, l.name AS list_name, \
             t.due_date, t.due_time, t.reminder_offsets_minutes, \
             COALESCE((SELECT offsets_minutes FROM task_reminder_preferences WHERE task_id = t.id AND user_id = $2), t.reminder_offsets_minutes) AS my_reminder_offsets_minutes, \
             t.done, t.assignee_id, assigned.email_normalized AS assignee_email, \
             s.name AS space_name, s.kind AS space_kind \
             FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
             JOIN spaces s ON s.id = l.space_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             LEFT JOIN LATERAL ( \
               SELECT email_normalized FROM email_identities \
               WHERE user_id = t.assignee_id ORDER BY created_at LIMIT 1 \
             ) assigned ON TRUE \
             WHERE t.id = $1",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(|_| TodoError::Unavailable)
    }

    async fn space_write_error(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<TodoError, TodoError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        Ok(if role.as_deref() == Some("viewer") {
            TodoError::Forbidden
        } else {
            TodoError::NotFound
        })
    }

    async fn task_write_error(&self, task_id: Uuid, user_id: Uuid) -> Result<TodoError, TodoError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT m.role FROM tasks t JOIN todo_lists l ON l.id = t.list_id \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             WHERE t.id = $1",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        Ok(if role.as_deref() == Some("viewer") {
            TodoError::Forbidden
        } else {
            TodoError::NotFound
        })
    }

    async fn list_write_error(&self, list_id: Uuid, user_id: Uuid) -> Result<TodoError, TodoError> {
        let role = sqlx::query_scalar::<_, String>(
            "SELECT m.role FROM todo_lists l \
             JOIN space_memberships m ON m.space_id = l.space_id AND m.user_id = $2 \
             WHERE l.id = $1",
        )
        .bind(list_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TodoError::Unavailable)?;
        Ok(if role.as_deref() == Some("viewer") {
            TodoError::Forbidden
        } else {
            TodoError::NotFound
        })
    }
}

#[derive(sqlx::FromRow)]
struct TaskListRow {
    id: Uuid,
    space_id: Uuid,
    name: String,
    space_name: String,
    space_kind: String,
}

impl From<TaskListRow> for TaskList {
    fn from(row: TaskListRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            name: row.name,
            space_name: row.space_name,
            space_kind: row.space_kind,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    space_id: Uuid,
    list_id: Uuid,
    title: String,
    list_name: String,
    due_date: Option<NaiveDate>,
    due_time: Option<NaiveTime>,
    reminder_offsets_minutes: Vec<i32>,
    my_reminder_offsets_minutes: Vec<i32>,
    done: bool,
    assignee_id: Option<Uuid>,
    assignee_email: Option<String>,
    space_name: String,
    space_kind: String,
}

impl From<TaskRow> for Task {
    fn from(row: TaskRow) -> Self {
        Self {
            id: row.id,
            space_id: row.space_id,
            list_id: row.list_id,
            title: row.title,
            list_name: row.list_name,
            due_date: row.due_date,
            due_time: row.due_time,
            reminder_offsets_minutes: row.reminder_offsets_minutes,
            my_reminder_offsets_minutes: row.my_reminder_offsets_minutes,
            done: row.done,
            assignee_id: row.assignee_id,
            assignee_email: row.assignee_email,
            space_name: row.space_name,
            space_kind: row.space_kind,
        }
    }
}
