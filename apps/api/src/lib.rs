pub mod auth;
pub mod events;
pub mod health;
pub mod logging;
pub mod members;
pub mod notifications;
pub mod pagination;
pub mod places;
pub mod spaces;
pub mod task_reminders;
pub mod time_zones;
pub mod todos;

#[cfg(debug_assertions)]
pub mod openapi;

pub fn database_connection_options() -> sqlx::postgres::PgConnectOptions {
    use std::env;
    sqlx::postgres::PgConnectOptions::new()
        .host(&env::var("POSTGRES_HOST").unwrap_or_else(|_| "db".into()))
        .username(&env::var("POSTGRES_USER").unwrap_or_else(|_| "horizon".into()))
        .password(&env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "local-only-password".into()))
        .database(&env::var("POSTGRES_DB").unwrap_or_else(|_| "horizon".into()))
}
