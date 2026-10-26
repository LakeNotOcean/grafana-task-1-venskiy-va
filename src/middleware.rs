use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::handlers::ApiError;
use crate::routes::AppState;

/// подключение метрик для маршрута
pub async fn observe(
    State(state): State<AppState>,
    matched: MatchedPath,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let handler = matched.as_str().to_owned();
    let started = Instant::now();

    let response = next.run(request).await;

    state.metrics.observe(
        &method,
        &handler,
        response.status().as_u16(),
        started.elapsed().as_secs_f64(),
    );

    response
}

/// имиитация сбоя
pub async fn fault_injection(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if state.is_scheduled_failure() {
        ApiError::ScheduledFailure.into_response()
    } else {
        next.run(request).await
    }
}
