use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;

use crate::config::Config;
use crate::handlers;
use crate::metrics::Metrics;
use crate::middleware;
use crate::store::Store;

/// jписание методов для  `GET /handlers`.
#[derive(Debug, Clone, Serialize)]
pub struct HandlerInfo {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

/// список методов для генератора
pub const HANDLERS: &[HandlerInfo] = &[
    HandlerInfo {
        method: "POST",
        path: "/items",
        description: "create",
    },
    HandlerInfo {
        method: "GET",
        path: "/items/{id}",
        description: "get",
    },
    HandlerInfo {
        method: "PUT",
        path: "/items/{id}",
        description: "get",
    },
];



#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<Store>,
    pub metrics: Arc<Metrics>,
    request_counter: Arc<AtomicU64>,
}

impl AppState {
    /// инициализируем состояние приложения с метриками
    pub fn new(config: Config) -> Result<Self, prometheus::Error> {
        let metrics = Metrics::new(&config.pod_id, &config.datacenter)?;

        Ok(Self {
            config: Arc::new(config),
            store: Arc::new(Store::new()),
            metrics: Arc::new(metrics),
            request_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    /// нужно ли моделировать сбой
    pub fn is_scheduled_failure(&self) -> bool {
        let every_n = self.config.fail_every_n;
        if every_n == 0 {
            return false;
        }

        let number = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        number % every_n == 0
    }
}

/// роутинг приложения
pub fn router(state: AppState) -> Router {
    // основные пути, с возможностью искусственного сбоя через middleware
    let routes = Router::new()
        .route("/items", post(handlers::create_item))
        .route(
            "/items/{id}",
            get(handlers::get_item).put(handlers::update_item),
        )
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::fault_injection,
        ))
        .route_layer(from_fn_with_state(state.clone(), middleware::observe));

    // служебные пути
    routes
        .route("/health", get(handlers::health))
        .route("/metrics", get(handlers::metrics))
        .route("/handlers", get(handlers::handlers_list))
        .with_state(state)
}
