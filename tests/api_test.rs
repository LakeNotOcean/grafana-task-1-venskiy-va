use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use items_service::config::Config;
use items_service::routes::{router, AppState};

fn test_config() -> Config {
    Config {
        port: 8080,
        pod_id: "pod-test".to_owned(),
        datacenter: "dc-test".to_owned(),
        fail_every_n: 0,
    }
}

fn app() -> Router {
    app_with(test_config())
}

fn app_with(config: Config) -> Router {
    router(AppState::new(config).expect("состояние приложения"))
}

fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .expect("запрос собран корректно")
}

async fn json_body(response: Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("тело ответа прочитано")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("в теле ответа JSON")
}

async fn create_item(app: Router, name: &str, quantity: u64) -> serde_json::Value {
    let body = format!(r#"{{"name":"{name}","quantity":{quantity}}}"#);
    let response = app
        .oneshot(json_request("POST", "/items", &body))
        .await
        .expect("запрос обработан");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

#[tokio::test]
async fn health_returns_ok() {
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("запрос собран корректно");

    let response = app().oneshot(request).await.expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn created_item_has_id_and_echoes_fields() {
    let created = create_item(app(), "widget", 7).await;

    assert_eq!(created["id"], 1);
    assert_eq!(created["name"], "widget");
    assert_eq!(created["quantity"], 7);
}

#[tokio::test]
async fn created_item_can_be_read_back() {
    let app = app();
    let created = create_item(app.clone(), "widget", 7).await;

    let response = app
        .clone()
        .oneshot(json_request("GET", "/items/1", ""))
        .await
        .expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, created);
}

#[tokio::test]
async fn missing_item_returns_404() {
    let response = app()
        .oneshot(json_request("GET", "/items/999", ""))
        .await
        .expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json_body(response).await["error"],
        "запись с id 999 не найдена"
    );
}

#[tokio::test]
async fn update_replaces_all_fields() {
    let app = app();
    create_item(app.clone(), "widget", 7).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/items/1",
            r#"{"name":"gadget","quantity":3}"#,
        ))
        .await
        .expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::OK);
    let updated = json_body(response).await;
    assert_eq!(updated["id"], 1);
    assert_eq!(updated["name"], "gadget");
    assert_eq!(updated["quantity"], 3);
}

#[tokio::test]
async fn update_of_missing_item_returns_404() {
    let response = app()
        .oneshot(json_request(
            "PUT",
            "/items/999",
            r#"{"name":"gadget","quantity":3}"#,
        ))
        .await
        .expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_json_returns_400() {
    let response = app()
        .oneshot(json_request("POST", "/items", "{не json"))
        .await
        .expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

async fn metrics_text(app: Router) -> String {
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .expect("запрос собран корректно");

    let response = app.oneshot(request).await.expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/plain; version=0.0.4; charset=utf-8"
    );

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("тело ответа прочитано")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("метрики в utf-8")
}

#[tokio::test]
async fn metrics_expose_counter_and_histogram_with_labels() {
    let app = app();
    create_item(app.clone(), "widget", 7).await;

    let text = metrics_text(app).await;

    assert!(text.contains("total_requests{"), "{text}");
    assert!(text.contains(r#"handler="/items""#), "{text}");
    assert!(text.contains(r#"code="201""#), "{text}");
    assert!(text.contains(r#"pod="pod-test""#), "{text}");
    assert!(text.contains(r#"dc="dc-test""#), "{text}");
    assert!(
        text.contains("request_duration_seconds_bucket{"),
        "{text}"
    );
}

#[tokio::test]
async fn metrics_use_route_template_not_actual_path() {
    let app = app();
    create_item(app.clone(), "widget", 7).await;

    let response = app
        .clone()
        .oneshot(json_request("GET", "/items/1", ""))
        .await
        .expect("запрос обработан");
    assert_eq!(response.status(), StatusCode::OK);

    let text = metrics_text(app).await;

    assert!(text.contains(r#"handler="/items/{id}""#), "{text}");
    assert!(!text.contains(r#"handler="/items/1""#), "{text}");
}

#[tokio::test]
async fn service_routes_are_absent_from_metrics() {
    let app = app();

    let health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("запрос собран корректно"),
        )
        .await
        .expect("запрос обработан");
    assert_eq!(health.status(), StatusCode::OK);

    let text = metrics_text(app).await;

    assert!(!text.contains(r#"handler="/health""#), "{text}");
    assert!(!text.contains(r#"handler="/metrics""#), "{text}");
    assert!(!text.contains(r#"handler="/handlers""#), "{text}");
}

#[tokio::test]
async fn handlers_list_reports_business_routes() {
    let request = Request::builder()
        .uri("/handlers")
        .body(Body::empty())
        .expect("запрос собран корректно");

    let response = app().oneshot(request).await.expect("запрос обработан");

    assert_eq!(response.status(), StatusCode::OK);

    let body = json_body(response).await;
    let handlers: Vec<String> = body["handlers"]
        .as_array()
        .expect("список ручек")
        .iter()
        .map(|handler| {
            format!(
                "{} {}",
                handler["method"].as_str().expect("method — строка"),
                handler["path"].as_str().expect("path — строка")
            )
        })
        .collect();

    assert_eq!(
        handlers,
        vec!["POST /items", "GET /items/{id}", "PUT /items/{id}"]
    );
}

#[tokio::test]
async fn every_nth_request_fails_with_500_and_is_measured() {
    let app = app_with(Config {
        fail_every_n: 2,
        ..test_config()
    });

    let first = create_item(app.clone(), "a", 1).await;
    assert_eq!(first["name"], "a");

    let second = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/items",
            r#"{"name":"b","quantity":2}"#,
        ))
        .await
        .expect("запрос обработан");

    assert_eq!(second.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        json_body(second).await["error"],
        "смоделированный сбой сервиса"
    );

    let text = metrics_text(app).await;

    assert!(text.contains(r#"code="500""#), "{text}");
    assert!(text.contains(r#"code="201""#), "{text}");
}

#[tokio::test]
async fn fault_injection_is_disabled_when_every_n_is_zero() {
    let app = app();

    for index in 0..5 {
        create_item(app.clone(), &format!("item-{index}"), index).await;
    }

    let text = metrics_text(app).await;

    assert!(!text.contains(r#"code="500""#), "{text}");
}
