use actix_web::{
    http::{header::AUTHORIZATION, StatusCode},
    test as actix_test, web, App,
};
use async_trait::async_trait;
use horizon_api::{
    auth::{self, AuthSettings, EmailSender, MagicLinkRequest, RedeemMagicLinkRequest},
    spaces::{self, CreateSpaceRequest, SpaceResponse},
    todos::{
        self, CreateTaskListRequest, CreateTaskRequest, TaskBoardResponse, TaskResponse,
        UpdateTaskCompletionRequest, UpdateTaskListRequest, UpdateTaskRequest,
    },
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

struct RecordingMailer(Arc<std::sync::Mutex<Option<String>>>);

#[async_trait]
impl EmailSender for RecordingMailer {
    async fn send_magic_link(&self, _recipient: &str, link: &str) -> Result<(), String> {
        *self.0.lock().expect("mailer lock should be available") = Some(link.to_owned());
        Ok(())
    }
}

#[actix_web::test]
async fn tasks_persist_in_lists_with_membership_scoped_reads_and_editor_writes() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(horizon_api::database_connection_options());
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migrations should apply");
    let mailer = RecordingMailer(Arc::new(std::sync::Mutex::new(None)));
    let auth_state = auth::AuthState::with_email_sender(
        pool.clone(),
        Arc::new(RecordingMailer(mailer.0.clone())),
        AuthSettings {
            jwt_secret: "test-only-todo-lists-key-is-32-bytes".to_owned(),
            issuer: "horizon-test".to_owned(),
            audience: "horizon-api-test".to_owned(),
            key_id: "test-v1".to_owned(),
            public_url: "https://horizon.localhost/auth/complete".to_owned(),
        },
    )
    .expect("test auth configuration should be valid");
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(auth_state))
            .app_data(web::Data::new(spaces::SpacesApplication::new(pool.clone())))
            .app_data(web::Data::new(todos::TodosApplication::new(pool.clone())))
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(spaces::create_shared_space)
            .service(todos::list_tasks)
            .service(todos::create_task_list)
            .service(todos::update_task_list)
            .service(todos::delete_task_list)
            .service(todos::create_task)
            .service(todos::update_task)
            .service(todos::delete_task)
            .service(todos::update_task_completion)
            .service(todos::update_my_task_reminders),
    )
    .await;

    let email = format!("{}@example.test", Uuid::new_v4());
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links")
        .set_json(MagicLinkRequest {
            email,
            time_zone: None,
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::ACCEPTED
    );
    let link = mailer
        .0
        .lock()
        .expect("mailer lock should be available")
        .clone()
        .expect("magic link should have been sent");
    let request = actix_test::TestRequest::post()
        .uri("/auth/magic-links/redeem")
        .set_json(RedeemMagicLinkRequest {
            token: link.split_once("#token=").unwrap().1.to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let session: auth::AuthenticationResponse = actix_test::read_body_json(response).await;
    let access_token = session.tokens.access_token;

    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateSpaceRequest {
            name: "To-do test Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let space: SpaceResponse = actix_test::read_body_json(response).await;

    let assigned_user_id = Uuid::new_v4();
    let assigned_email = format!("{assigned_user_id}@example.test");
    sqlx::query("INSERT INTO users (id) VALUES ($1)")
        .bind(assigned_user_id)
        .execute(&pool)
        .await
        .expect("test should create a Space Member account");
    sqlx::query("INSERT INTO email_identities (id, user_id, email_normalized, verified_at) VALUES ($1, $2, $3, now())")
        .bind(Uuid::new_v4())
        .bind(assigned_user_id)
        .bind(&assigned_email)
        .execute(&pool)
        .await
        .expect("test should create the Member email identity");
    sqlx::query(
        "INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(space.id)
    .bind(assigned_user_id)
    .execute(&pool)
    .await
    .expect("test should add the assignee to the Task Space");

    let request = actix_test::TestRequest::post()
        .uri("/spaces")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateSpaceRequest {
            name: "Other task Space".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let other_space: SpaceResponse = actix_test::read_body_json(response).await;
    let other_member_id = Uuid::new_v4();
    let other_email = format!("{other_member_id}@example.test");
    sqlx::query("INSERT INTO users (id) VALUES ($1)")
        .bind(other_member_id)
        .execute(&pool)
        .await
        .expect("test should create a different Space account");
    sqlx::query("INSERT INTO email_identities (id, user_id, email_normalized, verified_at) VALUES ($1, $2, $3, now())")
        .bind(Uuid::new_v4())
        .bind(other_member_id)
        .bind(&other_email)
        .execute(&pool)
        .await
        .expect("test should create the other email identity");
    sqlx::query(
        "INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(other_space.id)
    .bind(other_member_id)
    .execute(&pool)
    .await
    .expect("test should add the account to a different Space");

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/lists", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateTaskListRequest {
            name: "Weekend prep".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let list: todos::TaskListResponse = actix_test::read_body_json(response).await;

    let create_task = || CreateTaskRequest {
        title: "Buy groceries".to_owned(),
        list_name: "Weekend prep".to_owned(),
        due_date: Some("2026-12-12".to_owned()),
        due_time: Some("10:30".to_owned()),
        assignee_id: Some(assigned_user_id),
        reminder_offsets_minutes: vec![1440, 60, 15, 2880, 10080],
    };
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/tasks", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_task())
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let task: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(task.title, "Buy groceries");
    assert_eq!(task.list_name, "Weekend prep");
    assert_eq!(task.due_date.as_deref(), Some("2026-12-12"));
    assert_eq!(task.due_time.as_deref(), Some("10:30"));
    assert_eq!(task.assignee_id, Some(assigned_user_id));
    assert_eq!(
        task.assignee_email.as_deref(),
        Some(assigned_email.as_str())
    );
    assert!(!task.done);
    assert_eq!(
        task.reminder_offsets_minutes,
        vec![15, 60, 1440, 2880, 10080]
    );

    for offsets in [vec![30, 30], vec![5], vec![15, 30, 60, 120, 1440, 2880]] {
        let request = actix_test::TestRequest::post()
            .uri(&format!("/spaces/{}/tasks", space.id))
            .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
            .set_json(CreateTaskRequest {
                reminder_offsets_minutes: offsets,
                ..create_task()
            })
            .to_request();
        assert_eq!(
            actix_test::call_service(&app, request).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/tasks", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateTaskRequest {
            title: "Invalid cross-Space assignment".to_owned(),
            list_name: "Must not be created".to_owned(),
            due_date: None,
            due_time: None,
            assignee_id: Some(other_member_id),
            reminder_offsets_minutes: vec![1440, 60],
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::BAD_REQUEST
    );
    let orphaned_list_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM todo_lists WHERE space_id = $1 AND name = 'Must not be created'",
    )
    .bind(space.id)
    .fetch_one(&pool)
    .await
    .expect("test should query for an accidental list");
    assert_eq!(orphaned_list_count, 0);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/tasks", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(serde_json::json!({
            "title": "Pack snacks",
            "list_name": "Weekend prep",
            "due_date": "2026-12-13"
        }))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let second_task: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(second_task.title, "Pack snacks");
    assert_eq!(second_task.assignee_id, None);
    assert!(second_task.reminder_offsets_minutes.is_empty());

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}/reminders", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(todos::UpdateTaskReminderPreferenceRequest {
            offsets_minutes: vec![60],
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NOT_FOUND,
        "the creator cannot edit an assignee's private reminder choices"
    );

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}/reminders", second_task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(todos::UpdateTaskReminderPreferenceRequest {
            offsets_minutes: vec![30],
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let private_preference: todos::TaskReminderPreferenceResponse =
        actix_test::read_body_json(response).await;
    assert_eq!(private_preference.offsets_minutes, vec![30]);
    let (shared_offsets, private_offsets): (Vec<i32>, Vec<i32>) = sqlx::query_as(
        "SELECT t.reminder_offsets_minutes, p.offsets_minutes FROM tasks t \
         JOIN task_reminder_preferences p ON p.task_id = t.id AND p.user_id = t.creator_id \
         WHERE t.id = $1",
    )
    .bind(second_task.id)
    .fetch_one(&pool)
    .await
    .expect("test should read private and shared offsets independently");
    assert!(shared_offsets.is_empty());
    assert_eq!(private_offsets, vec![30]);

    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/lists", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateTaskListRequest {
            name: "Trip ideas".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CREATED
    );
    let request = actix_test::TestRequest::put()
        .uri(&format!("/lists/{}", list.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskListRequest {
            name: "Trip ideas".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::CONFLICT
    );

    let request = actix_test::TestRequest::get()
        .uri("/tasks?lists_limit=1&tasks_limit=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let board: TaskBoardResponse = actix_test::read_body_json(response).await;
    assert_eq!(board.lists.len(), 1);
    assert_eq!(board.tasks.len(), 1);
    assert_eq!(board.tasks[0].space_id, space.id);
    assert_eq!(board.tasks[0].assignee_id, Some(assigned_user_id));
    assert_eq!(board.lists_next_offset, Some(1));
    assert_eq!(board.tasks_next_offset, Some(1));
    let request = actix_test::TestRequest::get()
        .uri("/tasks?lists_limit=1&tasks_limit=1&lists_offset=1&tasks_offset=1")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    let next_board: TaskBoardResponse = actix_test::read_body_json(response).await;
    assert_eq!(next_board.lists.len(), 1);
    assert_eq!(next_board.tasks.len(), 1);
    assert_eq!(next_board.lists_next_offset, None);
    assert_eq!(next_board.tasks_next_offset, None);
    assert_ne!(board.tasks[0].id, next_board.tasks[0].id);

    sqlx::query(
        "UPDATE space_memberships SET role = 'editor' WHERE space_id = $1 AND user_id = $2",
    )
    .bind(space.id)
    .bind(session.account.id)
    .execute(&pool)
    .await
    .expect("test should be able to switch the member to Editor");
    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}/completion", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskCompletionRequest { done: true })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let completed: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert!(completed.done);

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskRequest {
            title: "Buy groceries and fruit".to_owned(),
            list_name: "Errands".to_owned(),
            due_date: Some("2026-12-14".to_owned()),
            due_time: Some("11:30".to_owned()),
            assignee_id: Some(assigned_user_id),
            reminder_offsets_minutes: vec![1440, 60],
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(updated.title, "Buy groceries and fruit");
    assert_eq!(updated.list_name, "Errands");
    assert_eq!(updated.due_date.as_deref(), Some("2026-12-14"));
    assert_eq!(updated.due_time.as_deref(), Some("11:30"));
    assert_eq!(updated.assignee_id, Some(assigned_user_id));

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskRequest {
            title: "Buy groceries and fruit".to_owned(),
            list_name: "Errands".to_owned(),
            due_date: Some("2026-12-14".to_owned()),
            due_time: Some("11:30".to_owned()),
            assignee_id: None,
            reminder_offsets_minutes: vec![1440, 60],
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let unassigned: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(unassigned.assignee_id, None);

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskRequest {
            title: "Buy groceries and fruit".to_owned(),
            list_name: "Errands".to_owned(),
            due_date: Some("2026-12-14".to_owned()),
            due_time: Some("11:30".to_owned()),
            assignee_id: Some(assigned_user_id),
            reminder_offsets_minutes: vec![1440, 60],
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let reassigned: todos::TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(reassigned.assignee_id, Some(assigned_user_id));

    let request = actix_test::TestRequest::put()
        .uri(&format!("/lists/{}", list.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskListRequest {
            name: "Weekend chores".to_owned(),
        })
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    sqlx::query(
        "UPDATE space_memberships SET role = 'viewer' WHERE space_id = $1 AND user_id = $2",
    )
    .bind(space.id)
    .bind(session.account.id)
    .execute(&pool)
    .await
    .expect("test should be able to switch the member to Viewer");
    let request = actix_test::TestRequest::get()
        .uri("/tasks")
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let viewer_board: TaskBoardResponse = actix_test::read_body_json(response).await;
    assert_eq!(viewer_board.tasks.len(), 2);
    assert!(viewer_board.tasks.iter().any(|viewer_task| {
        viewer_task.id == task.id
            && viewer_task.done
            && viewer_task.assignee_email.as_deref() == Some(assigned_email.as_str())
    }));

    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}/completion", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskCompletionRequest { done: false })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::put()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(UpdateTaskRequest {
            title: "Viewer cannot update".to_owned(),
            list_name: "Errands".to_owned(),
            due_date: None,
            due_time: None,
            assignee_id: None,
            reminder_offsets_minutes: vec![1440, 60],
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/lists/{}", list.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/lists", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(CreateTaskListRequest {
            name: "Blocked by Viewer".to_owned(),
        })
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = actix_test::TestRequest::post()
        .uri(&format!("/spaces/{}/tasks", space.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .set_json(create_task())
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );

    sqlx::query("UPDATE space_memberships SET role = 'owner' WHERE space_id = $1 AND user_id = $2")
        .bind(space.id)
        .bind(session.account.id)
        .execute(&pool)
        .await
        .expect("test should restore the Owner role for destructive coverage");
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/tasks/{}", task.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let deleted_task_count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks WHERE id = $1")
            .bind(task.id)
            .fetch_one(&pool)
            .await
            .expect("test should verify Task deletion");
    assert_eq!(deleted_task_count, 0);
    let request = actix_test::TestRequest::delete()
        .uri(&format!("/lists/{}", list.id))
        .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
    let cascaded_task_count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks WHERE id = $1")
            .bind(task.id)
            .fetch_one(&pool)
            .await
            .expect("test should verify cascade deletion");
    assert_eq!(cascaded_task_count, 0);

    let post_to_list = |title: &str| {
        actix_test::TestRequest::post()
            .uri(&format!("/spaces/{}/tasks", space.id))
            .insert_header((AUTHORIZATION, format!("Bearer {access_token}")))
            .set_json(CreateTaskRequest {
                title: title.to_owned(),
                list_name: "Concurrent To-dos".to_owned(),
                assignee_id: None,
                ..create_task()
            })
            .to_request()
    };
    let (first, second) = tokio::join!(
        actix_test::call_service(&app, post_to_list("First concurrent Task")),
        actix_test::call_service(&app, post_to_list("Second concurrent Task")),
    );
    assert_eq!(first.status(), StatusCode::CREATED);
    assert_eq!(second.status(), StatusCode::CREATED);
    let first: TaskResponse = actix_test::read_body_json(first).await;
    let second: TaskResponse = actix_test::read_body_json(second).await;
    assert_ne!(first.id, second.id);
    assert_eq!(first.list_id, second.list_id);

    let list_version = |list_id: Uuid| {
        sqlx::query_scalar::<_, String>("SELECT xmin::text FROM todo_lists WHERE id = $1")
            .bind(list_id)
            .fetch_one(&pool)
    };
    let before = list_version(first.list_id).await.unwrap();
    let response =
        actix_test::call_service(&app, post_to_list("Another Task in the same list")).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let task: TaskResponse = actix_test::read_body_json(response).await;
    assert_eq!(task.list_id, first.list_id);
    assert_eq!(list_version(first.list_id).await.unwrap(), before);

    let mut deletion = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM todo_lists WHERE id = $1")
        .bind(first.list_id)
        .execute(&mut *deletion)
        .await
        .unwrap();
    let creation = actix_test::call_service(&app, post_to_list("Task in replacement list"));
    tokio::pin!(creation);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut creation)
            .await
            .is_err(),
        "Task creation must wait for an in-flight list deletion"
    );
    deletion.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), creation)
        .await
        .expect("Task creation should continue after deletion commits");
    assert_eq!(response.status(), StatusCode::CREATED);
    let replacement: TaskResponse = actix_test::read_body_json(response).await;
    assert_ne!(replacement.list_id, first.list_id);
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks WHERE list_id = $1")
        .bind(first.list_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    let _ = sqlx::query("DELETE FROM spaces WHERE id = $1")
        .bind(space.id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM spaces WHERE id = $1")
        .bind(other_space.id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1 OR id = $2")
        .bind(assigned_user_id)
        .bind(other_member_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(session.account.id)
        .execute(&pool)
        .await;
}
