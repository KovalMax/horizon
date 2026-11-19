use actix_web::{get, web, HttpResponse, Responder};
use sqlx::PgPool;

#[utoipa::path(
    get,
    path = "/health/live",
    responses((status = 200, description = "API process is running", body = String))
)]
#[get("/health/live")]
pub async fn liveness() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body("API process is running")
}

#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "API and database are available", body = String),
        (status = 503, description = "Database is unavailable", body = String)
    )
)]
#[get("/health/ready")]
pub async fn readiness(pool: web::Data<PgPool>) -> impl Responder {
    match sqlx::query("SELECT 1").execute(pool.get_ref()).await {
        Ok(_) => HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body("Database is available"),
        Err(error) => {
            log::error!(target: "horizon_api::database", "readiness query failed: {error}");
            HttpResponse::ServiceUnavailable()
                .content_type("text/plain; charset=utf-8")
                .body("Database is unavailable")
        }
    }
}
