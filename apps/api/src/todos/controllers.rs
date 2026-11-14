use actix_web::{
    delete, get,
    http::{header, StatusCode},
    post, put, web, HttpRequest, HttpResponse,
};
use uuid::Uuid;

use crate::auth::{AuthApplication, AuthError, Problem};
use crate::pagination::{PageWindow, TaskBoardPageQuery};

use super::{
    models::TodoError,
    requests::{
        CreateTaskListRequest, CreateTaskRequest, UpdateTaskCompletionRequest,
        UpdateTaskListRequest, UpdateTaskReminderPreferenceRequest, UpdateTaskRequest,
    },
    services::TodosApplication,
};

fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

async fn authenticated_user(
    auth: &AuthApplication,
    request: &HttpRequest,
) -> Result<Uuid, AuthenticationFailure> {
    let Some(token) = bearer_token(request) else {
        return Err(AuthenticationFailure::Unauthorized);
    };
    auth.current_account(token)
        .await
        .map(|account| account.id)
        .map_err(|error| match error {
            AuthError::Unauthorized => AuthenticationFailure::Unauthorized,
            _ => AuthenticationFailure::Unavailable,
        })
}

enum AuthenticationFailure {
    Unauthorized,
    Unavailable,
}

impl AuthenticationFailure {
    fn into_response(self) -> HttpResponse {
        match self {
            Self::Unauthorized => problem(
                StatusCode::UNAUTHORIZED,
                "Authentication failed",
                "authentication_failed",
            ),
            Self::Unavailable => problem(
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable",
                "database_unavailable",
            ),
        }
    }
}

fn problem(status: StatusCode, title: &'static str, code: &'static str) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title,
            status: status.as_u16(),
            code,
        })
}

fn todo_error(error: TodoError) -> HttpResponse {
    match error {
        TodoError::Invalid => problem(
            StatusCode::BAD_REQUEST,
            "Invalid To-do item",
            "invalid_todo_item",
        ),
        TodoError::Conflict => problem(
            StatusCode::CONFLICT,
            "A To-do list with that name already exists",
            "todo_list_name_conflict",
        ),
        TodoError::Forbidden => problem(
            StatusCode::FORBIDDEN,
            "Insufficient Space role",
            "insufficient_space_role",
        ),
        TodoError::NotFound => problem(
            StatusCode::NOT_FOUND,
            "To-do item not found",
            "todo_item_not_found",
        ),
        TodoError::Unavailable => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    get,
    path = "/tasks",
    params(TaskBoardPageQuery),
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "To-do lists and Tasks in Spaces the Member can access", body = super::responses::TaskBoardResponse),
        (status = 400, description = "Page offset is invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[get("/tasks")]
pub async fn list_tasks(
    request: HttpRequest,
    query: web::Query<TaskBoardPageQuery>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    let Some(lists_page) = PageWindow::new(query.lists_offset, query.lists_limit) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid lists offset",
            "invalid_page_offset",
        );
    };
    let Some(tasks_page) = PageWindow::new(query.tasks_offset, query.tasks_limit) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "Invalid Tasks offset",
            "invalid_page_offset",
        );
    };
    match todos.list_for_member(user_id, lists_page, tasks_page).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/spaces/{space_id}/lists",
    params(("space_id" = Uuid, Path, description = "Space containing the list")),
    request_body = CreateTaskListRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 201, description = "To-do list created", body = super::responses::TaskListResponse),
        (status = 400, description = "List name is invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot create lists", body = Problem),
        (status = 404, description = "Space not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[post("/spaces/{space_id}/lists")]
pub async fn create_task_list(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<CreateTaskListRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.create_list(path.into_inner(), user_id, &body).await {
        Ok(list) => HttpResponse::Created().json(list),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/lists/{list_id}",
    params(("list_id" = Uuid, Path, description = "List to rename")),
    request_body = UpdateTaskListRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "To-do list renamed", body = super::responses::TaskListResponse),
        (status = 400, description = "List name is invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot rename lists", body = Problem),
        (status = 404, description = "List not found", body = Problem),
        (status = 409, description = "A list with that name already exists", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[put("/lists/{list_id}")]
pub async fn update_task_list(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<UpdateTaskListRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.update_list(path.into_inner(), user_id, &body).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    delete,
    path = "/lists/{list_id}",
    params(("list_id" = Uuid, Path, description = "List and its Tasks to delete")),
    security(("accessBearer" = [])),
    responses(
        (status = 204, description = "List and its Tasks deleted"),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot delete lists", body = Problem),
        (status = 404, description = "List not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[delete("/lists/{list_id}")]
pub async fn delete_task_list(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.delete_list(path.into_inner(), user_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/spaces/{space_id}/tasks",
    params(("space_id" = Uuid, Path, description = "Space containing the Task")),
    request_body = CreateTaskRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 201, description = "Task created", body = super::responses::TaskResponse),
        (status = 400, description = "Task fields are invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot create Tasks", body = Problem),
        (status = 404, description = "Space not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[post("/spaces/{space_id}/tasks")]
pub async fn create_task(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<CreateTaskRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.create_task(path.into_inner(), user_id, &body).await {
        Ok(task) => HttpResponse::Created().json(task),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/tasks/{task_id}",
    params(("task_id" = Uuid, Path, description = "Task to update")),
    request_body = UpdateTaskRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "Task updated", body = super::responses::TaskResponse),
        (status = 400, description = "Task fields or assignee are invalid", body = Problem),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot update Tasks", body = Problem),
        (status = 404, description = "Task not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[put("/tasks/{task_id}")]
pub async fn update_task(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<UpdateTaskRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.update_task(path.into_inner(), user_id, &body).await {
        Ok(task) => HttpResponse::Ok().json(task),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    delete,
    path = "/tasks/{task_id}",
    params(("task_id" = Uuid, Path, description = "Task to delete")),
    security(("accessBearer" = [])),
    responses(
        (status = 204, description = "Task deleted"),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot delete Tasks", body = Problem),
        (status = 404, description = "Task not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[delete("/tasks/{task_id}")]
pub async fn delete_task(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos.delete_task(path.into_inner(), user_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/tasks/{task_id}/completion",
    params(("task_id" = Uuid, Path, description = "Task to mark open or completed")),
    request_body = UpdateTaskCompletionRequest,
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "Task completion state updated", body = super::responses::TaskResponse),
        (status = 401, description = "Access token is invalid or missing", body = Problem),
        (status = 403, description = "Viewer cannot update Tasks", body = Problem),
        (status = 404, description = "Task not found", body = Problem),
        (status = 503, description = "Service unavailable", body = Problem)
    )
)]
#[put("/tasks/{task_id}/completion")]
pub async fn update_task_completion(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<UpdateTaskCompletionRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos
        .update_completion(path.into_inner(), user_id, body.done)
        .await
    {
        Ok(task) => HttpResponse::Ok().json(task),
        Err(error) => todo_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/tasks/{task_id}/reminders",
    params(("task_id" = Uuid, Path, description = "Task to change the caller's reminder offsets for")),
    request_body = UpdateTaskReminderPreferenceRequest,
    security(("accessBearer" = [])),
    responses((status = 200, body = super::responses::TaskReminderPreferenceResponse), (status = 400, body = Problem), (status = 401, body = Problem), (status = 404, body = Problem))
)]
#[put("/tasks/{task_id}/reminders")]
pub async fn update_my_task_reminders(
    request: HttpRequest,
    path: web::Path<Uuid>,
    auth: web::Data<AuthApplication>,
    todos: web::Data<TodosApplication>,
    body: web::Json<UpdateTaskReminderPreferenceRequest>,
) -> HttpResponse {
    let user_id = match authenticated_user(&auth, &request).await {
        Ok(user_id) => user_id,
        Err(error) => return error.into_response(),
    };
    match todos
        .set_my_reminder_preference(path.into_inner(), user_id, &body.offsets_minutes)
        .await
    {
        Ok(()) => HttpResponse::Ok().json(super::responses::TaskReminderPreferenceResponse {
            offsets_minutes: body.offsets_minutes.clone(),
        }),
        Err(error) => todo_error(error),
    }
}
