#![cfg(debug_assertions)]

use actix_web::{test, App};
use horizon_api::openapi::ApiDoc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::test]
async fn generated_openapi_document_is_served() {
    let app = test::init_service(
        App::new()
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/openapi.json", ApiDoc::openapi())),
    )
    .await;
    let ui_response = test::call_service(
        &app,
        test::TestRequest::get().uri("/swagger-ui/").to_request(),
    )
    .await;
    assert_eq!(ui_response.status(), actix_web::http::StatusCode::OK);
    assert_eq!(
        ui_response
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/html")
    );
    let ui_body = test::read_body(ui_response).await;
    assert!(!ui_body.is_empty());
    assert!(String::from_utf8_lossy(&ui_body).contains("Swagger UI"));

    let response = test::call_service(
        &app,
        test::TestRequest::get().uri("/openapi.json").to_request(),
    )
    .await;
    assert_eq!(response.status(), actix_web::http::StatusCode::OK);
    let body = test::read_body(response).await;
    let document = std::str::from_utf8(&body).expect("OpenAPI document should be UTF-8");
    assert!(document.contains("/health/live"));
    assert!(document.contains("/health/ready"));
    assert!(document.contains("/auth/magic-links"));
    assert!(document.contains("accessBearer"));
    assert!(document.contains("refreshBearer"));
}
