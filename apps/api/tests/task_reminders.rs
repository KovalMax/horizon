use async_trait::async_trait;
use horizon_api::{
    auth::EmailSender, database_connection_options, task_reminders::TaskReminderWorker,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingMailer(Arc<Mutex<Vec<String>>>);

#[async_trait]
impl EmailSender for RecordingMailer {
    async fn send_magic_link(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }

    async fn send_task_reminder(
        &self,
        _: &str,
        task_title: &str,
        _: &str,
        _: &str,
    ) -> Result<(), String> {
        let mut attempts = self.0.lock().unwrap();
        attempts.push(task_title.to_owned());
        if attempts.len() == 1 {
            Err("temporary SMTP failure".to_owned())
        } else {
            Ok(())
        }
    }
}

#[actix_web::test]
async fn worker_delivers_due_email_once_and_records_in_app_notification() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(database_connection_options());
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations should apply");

    let user_id = Uuid::new_v4();
    let space_id = Uuid::new_v4();
    let list_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let email = format!("{user_id}@example.test");
    sqlx::query("INSERT INTO users (id) VALUES ($1)")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO email_identities (id, user_id, email_normalized, verified_at) VALUES ($1, $2, $3, now())")
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&email)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO spaces (id, name, kind, personal_owner_id) VALUES ($1, 'Reminder test', 'personal', $2)")
        .bind(space_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(space_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO todo_lists (id, space_id, name) VALUES ($1, $2, 'Reminders')")
        .bind(list_id)
        .bind(space_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tasks (id, list_id, creator_id, title, due_date, due_time, reminder_offsets_minutes) \
         SELECT $1, $2, $3, 'Check the greenhouse', (now() + interval '62 minutes')::date, \
           (now() + interval '62 minutes')::time, ARRAY[60]::integer[]",
    )
    .bind(task_id)
    .bind(list_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO notification_preferences (user_id, event_type, channel, enabled) \
         VALUES ($1, 'due_reminder', 'email', TRUE)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let endpoint = format!("https://fcm.googleapis.com/fcm/send/{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO web_push_subscriptions \
         (endpoint, user_id, p256dh, auth_secret, calendar_reminders, task_reminders) \
         VALUES ($1, $2, $3, $4, FALSE, TRUE)",
    )
    .bind(&endpoint)
    .bind(user_id)
    .bind(vec![0u8; 65])
    .bind(vec![0u8; 16])
    .execute(&pool)
    .await
    .unwrap();
    let mailer = RecordingMailer::default();
    let worker = TaskReminderWorker::new(pool.clone(), Arc::new(mailer.clone()));
    let test_now = chrono::Utc::now() + chrono::Duration::minutes(3);
    worker
        .run_once_at(test_now)
        .await
        .expect("worker should run");
    worker
        .run_once_at(test_now)
        .await
        .expect("worker should retry temporary SMTP failures");
    worker
        .run_once_at(test_now)
        .await
        .expect("successful jobs should not repeat");

    assert_eq!(
        mailer.0.lock().unwrap().as_slice(),
        &["Check the greenhouse", "Check the greenhouse"]
    );
    let inbox_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notifications WHERE recipient_id = $1 AND event_type = 'due_reminder'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(inbox_count, 1);
    let sent_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM task_reminder_jobs WHERE task_id = $1 AND status = 'sent'",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sent_count, 1);
    let queued_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_push_outbox WHERE endpoint = $1")
            .bind(&endpoint)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(queued_count, 1);

    let date_only_task = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tasks (id, list_id, creator_id, title, due_date, reminder_offsets_minutes) \
         VALUES ($1, $2, $3, 'Date-only reminders', CURRENT_DATE + 21, ARRAY[30, 2880, 10080])",
    )
    .bind(date_only_task)
    .bind(list_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();
    worker.run_once().await.unwrap();
    let date_only_jobs: Vec<(i32, String)> = sqlx::query_as(
        "SELECT offset_minutes, to_char(scheduled_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') \
         FROM task_reminder_jobs WHERE task_id = $1 ORDER BY offset_minutes",
    )
    .bind(date_only_task)
    .fetch_all(&pool)
    .await
    .unwrap();
    let date: chrono::NaiveDate = sqlx::query_scalar("SELECT due_date FROM tasks WHERE id = $1")
        .bind(date_only_task)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        date_only_jobs,
        vec![
            (30, format!("{date} 08:30")),
            (2880, format!("{} 09:00", date - chrono::Days::new(2))),
            (10080, format!("{} 09:00", date - chrono::Days::new(7))),
        ]
    );

    let lifecycle_task_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tasks (id, list_id, creator_id, title, due_date, due_time, reminder_offsets_minutes) \
         SELECT $1, $2, $3, 'Reopened task', (now() + interval '4 hours')::date, \
           (now() + interval '4 hours')::time, ARRAY[60]::integer[]",
    )
    .bind(lifecycle_task_id)
    .bind(list_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();
    worker
        .run_once()
        .await
        .expect("future reminder should be queued");
    sqlx::query("UPDATE tasks SET done = TRUE WHERE id = $1")
        .bind(lifecycle_task_id)
        .execute(&pool)
        .await
        .unwrap();
    worker
        .run_once()
        .await
        .expect("completion should cancel pending reminders");
    let cancelled: String =
        sqlx::query_scalar("SELECT status FROM task_reminder_jobs WHERE task_id = $1")
            .bind(lifecycle_task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cancelled, "cancelled");

    sqlx::query(
        "UPDATE tasks SET done = FALSE, due_date = (now() + interval '20 minutes')::date, \
         due_time = (now() + interval '20 minutes')::time WHERE id = $1",
    )
    .bind(lifecycle_task_id)
    .execute(&pool)
    .await
    .unwrap();
    worker
        .run_once()
        .await
        .expect("reopening must not backfill a missed reminder");
    let still_cancelled: String =
        sqlx::query_scalar("SELECT status FROM task_reminder_jobs WHERE task_id = $1")
            .bind(lifecycle_task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(still_cancelled, "cancelled");

    sqlx::query(
        "UPDATE tasks SET due_date = (now() + interval '4 hours')::date, \
         due_time = (now() + interval '4 hours')::time WHERE id = $1",
    )
    .bind(lifecycle_task_id)
    .execute(&pool)
    .await
    .unwrap();
    worker
        .run_once()
        .await
        .expect("future schedule edits should reactivate the reminder");
    let rescheduled: String =
        sqlx::query_scalar("SELECT status FROM task_reminder_jobs WHERE task_id = $1")
            .bind(lifecycle_task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rescheduled, "pending");

    sqlx::query(
        "INSERT INTO space_notification_preferences (user_id, space_id, muted) VALUES ($1, $2, TRUE) \
         ON CONFLICT (user_id, space_id) DO UPDATE SET muted = TRUE",
    )
    .bind(user_id)
    .bind(space_id)
    .execute(&pool)
    .await
    .unwrap();
    worker
        .run_once()
        .await
        .expect("muting should cancel pending reminders");
    sqlx::query(
        "UPDATE tasks SET due_date = (now() + interval '20 minutes')::date, \
         due_time = (now() + interval '20 minutes')::time WHERE id = $1",
    )
    .bind(lifecycle_task_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM space_notification_preferences WHERE user_id = $1 AND space_id = $2")
        .bind(user_id)
        .bind(space_id)
        .execute(&pool)
        .await
        .unwrap();
    worker
        .run_once()
        .await
        .expect("unmuting must not backfill an elapsed reminder");
    let muted_reminder: String =
        sqlx::query_scalar("SELECT status FROM task_reminder_jobs WHERE task_id = $1")
            .bind(lifecycle_task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(muted_reminder, "cancelled");
}
