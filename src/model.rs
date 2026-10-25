use serde::{Deserialize, Serialize};

/// Запись в хранилище.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub quantity: u64,
}

/// Тело `POST /items`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub quantity: u64,
}

/// Тело `PUT /items/{id}` — полная замена полей записи.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UpdateItemRequest {
    pub name: String,
    pub quantity: u64,
}
