use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::metrics::TEXT_CONTENT_TYPE;
use crate::model::{CreateItemRequest, Item, UpdateItemRequest};
use crate::routes::{AppState, HandlerInfo, HANDLERS};

/// Ошибки API
#[derive(Debug)]
pub enum ApiError {
    NotFound {
        id: u64,
    },
    /// Смоделированный сбой: нужен, чтобы на дашборде были видны 5xx.
    ScheduledFailure,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound { id } => (
                StatusCode::NOT_FOUND,
                format!("запись с id {id} не найдена"),
            ),
            ApiError::ScheduledFailure => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "смоделированный сбой сервиса".to_owned(),
            ),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// далее всякие обработчики

/// `GET /health` просто проверка
pub async fn health() -> StatusCode {
    StatusCode::OK
}

/// `POST /items`
pub async fn create_item(
    State(state): State<AppState>,
    Json(request): Json<CreateItemRequest>,
) -> (StatusCode, Json<Item>) {
    let item = state.store.create(request);
    (StatusCode::CREATED, Json(item))
}

/// `GET /items/{id}`
pub async fn get_item(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Item>, ApiError> {
    state
        .store
        .get(id)
        .map(Json)
        .ok_or(ApiError::NotFound { id })
}

/// `PUT /items/{id}`
pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(request): Json<UpdateItemRequest>,
) -> Result<Json<Item>, ApiError> {
    state
        .store
        .update(id, request)
        .map(Json)
        .ok_or(ApiError::NotFound { id })
}

/// `GET /metrics`
pub async fn metrics(State(state): State<AppState>) -> Response {
    match state.metrics.encode() {
        Ok(body) => ([(header::CONTENT_TYPE, TEXT_CONTENT_TYPE)], body).into_response(),
        Err(error) => {
            tracing::error!(%error, "не удалось экспортировать метрики");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Serialize)]
pub struct HandlersResponse {
    pub handlers: &'static [HandlerInfo],
}

/// `GET /handlers` список методов для генератора
pub async fn handlers_list() -> Json<HandlersResponse> {
    Json(HandlersResponse { handlers: HANDLERS })
}
