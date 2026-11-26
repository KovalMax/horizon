use horizon_api::{database_connection_options, notifications::PushWorker};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[actix_web::test]
async fn outbox_cleanup_bounds_each_batch_and_leaves_current_jobs() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_lazy_with(database_connection_options());
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let user = Uuid::new_v4();
    let space = Uuid::new_v4();
    let endpoint = format!("https://example.test/{user}");
    sqlx::query("INSERT INTO users (id) VALUES ($1)")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO spaces (id, name, kind) VALUES ($1, 'Retention test', 'shared')")
        .bind(space)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO web_push_subscriptions \
         (endpoint, user_id, p256dh, auth_secret, calendar_reminders, task_reminders) \
         VALUES ($1, $2, $3, $4, TRUE, TRUE)",
    )
    .bind(&endpoint)
    .bind(user)
    .bind(vec![0_u8; 65])
    .bind(vec![0_u8; 16])
    .execute(&pool)
    .await
    .unwrap();

    let now: chrono::DateTime<chrono::Utc> = "2000-01-04T00:00:00Z".parse().unwrap();
    sqlx::query(
        "INSERT INTO web_push_outbox \
         (endpoint, space_id, reminder_key, kind, item_id, reference_time, offset_minutes, expires_at, finished) \
         SELECT $1, $2, n::text, 'tasks', gen_random_uuid(), $3, 60, \
                CASE WHEN n <= 502 THEN $3::timestamptz - interval '2 days' \
                     WHEN n = 503 THEN $3::timestamptz - interval '1 day' ELSE $3::timestamptz + interval '1 hour' END, \
                n % 2 = 0 \
         FROM generate_series(1, 504) n",
    )
    .bind(&endpoint)
    .bind(space)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let mut held = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT id FROM web_push_outbox WHERE endpoint = $1 AND reminder_key = '1' FOR UPDATE",
    )
    .bind(&endpoint)
    .fetch_one(&mut *held)
    .await
    .unwrap();
    let run_cleanup = || {
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            PushWorker::purge_expired_batch(&pool, now),
        )
    };
    assert_eq!(run_cleanup().await.unwrap().unwrap(), 500);
    assert_eq!(run_cleanup().await.unwrap().unwrap(), 1);
    held.rollback().await.unwrap();
    assert_eq!(run_cleanup().await.unwrap().unwrap(), 1);
    assert_eq!(run_cleanup().await.unwrap().unwrap(), 0);

    let remaining: Vec<String> = sqlx::query_scalar(
        "SELECT reminder_key FROM web_push_outbox WHERE endpoint = $1 ORDER BY reminder_key",
    )
    .bind(&endpoint)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, ["503", "504"]);
}
