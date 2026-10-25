use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::model::{CreateItemRequest, Item, UpdateItemRequest};

/// In-memory хранилище записей. Id выдаются монотонно, начиная с 1.
#[derive(Debug, Default)]
pub struct Store {
    items: RwLock<HashMap<u64, Item>>,
    next_id: AtomicU64,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&self, request: CreateItemRequest) -> Item {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let item = Item {
            id,
            name: request.name,
            quantity: request.quantity,
        };
        self.items_write().insert(id, item.clone());
        item
    }

    pub fn get(&self, id: u64) -> Option<Item> {
        self.items_read().get(&id).cloned()
    }

    pub fn update(&self, id: u64, request: UpdateItemRequest) -> Option<Item> {
        let mut items = self.items_write();
        let item = items.get_mut(&id)?;
        item.name = request.name;
        item.quantity = request.quantity;
        Some(item.clone())
    }

    // Отравленная блокировка означает панику в другом потоке; сами данные остаются
    // валидными, поэтому продолжаем работу вместо второй паники.
    fn items_read(&self) -> RwLockReadGuard<'_, HashMap<u64, Item>> {
        self.items
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn items_write(&self) -> RwLockWriteGuard<'_, HashMap<u64, Item>> {
        self.items
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(name: &str, quantity: u64) -> CreateItemRequest {
        CreateItemRequest {
            name: name.to_owned(),
            quantity,
        }
    }

    fn update(name: &str, quantity: u64) -> UpdateItemRequest {
        UpdateItemRequest {
            name: name.to_owned(),
            quantity,
        }
    }

    #[test]
    fn create_assigns_ids_starting_from_one() {
        let store = Store::new();

        let first = store.create(create("a", 1));
        let second = store.create(create("b", 2));

        assert_eq!(first.id, 1);
        assert_eq!(second.id, 2);
    }

    #[test]
    fn get_returns_stored_item() {
        let store = Store::new();
        let created = store.create(create("widget", 7));

        assert_eq!(store.get(created.id), Some(created));
        assert_eq!(store.get(999), None);
    }

    #[test]
    fn update_replaces_all_fields() {
        let store = Store::new();
        let created = store.create(create("widget", 7));

        let updated = store.update(created.id, update("gadget", 3));

        assert_eq!(
            updated,
            Some(Item {
                id: created.id,
                name: "gadget".to_owned(),
                quantity: 3,
            })
        );
        assert_eq!(store.get(created.id), updated);
    }

    #[test]
    fn update_of_missing_item_returns_none() {
        let store = Store::new();

        assert_eq!(store.update(42, update("x", 1)), None);
    }
}
