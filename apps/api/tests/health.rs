use actix_web::{http::StatusCode, test, web, App};
use horizon_api::{database_connection_options, health};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::time::Duration;

#[actix_web::test]
async fn readiness_reports_unavailable_database() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(250))
        .connect_lazy_with(PgConnectOptions::new().host("127.0.0.1").port(1));
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .service(health::readiness),
    )
    .await;
    let response = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(test::read_body(response).await, "Database is unavailable");
}

#[actix_web::test]
async fn liveness_response_has_a_text_content_type() {
    let app = test::init_service(App::new().service(health::liveness)).await;
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/health/live")
            .insert_header((actix_web::http::header::ACCEPT, "text/plain"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
}

#[actix_web::test]
async fn readiness_reports_real_database() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(database_connection_options());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .service(health::readiness),
    )
    .await;
    let response = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}
