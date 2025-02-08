use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tracing::error;

use crate::{item::StatusNotifierItem, menu::TrayMenu};

type BaseMap = HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>;

#[derive(Debug, Clone)]
pub struct TrayItemMap {
    inner: Arc<Mutex<BaseMap>>,
}

impl TrayItemMap {
    pub fn get_map(&self) -> Arc<Mutex<BaseMap>> {
        self.inner.clone()
    }

    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub(super) fn new_item(&self, dest: String, item: StatusNotifierItem) {
        self.inner
            .lock()
            .expect("mutex lock should succeed")
            .insert(dest, (item, None));
    }
    pub(super) fn remove_item(&self, dest: &str) {
        self.inner
            .lock()
            .expect("mutex lock should succeed")
            .remove(dest);
    }
    pub(super) fn update_menu(&self, dest: &str, menu: &TrayMenu) {
        if let Some((_, menu_cache)) = self
            .inner
            .lock()
            .expect("mutex lock should succeed")
            .get_mut(dest)
        {
            menu_cache.replace(menu.clone());
        } else {
            error!("could not find item in state");
        }
    }
    pub(super) fn clear_items(&self) -> Vec<String> {
        let mut items = self.inner.lock().expect("mutex lock should succeed");
        let keys = items.keys().cloned().collect::<Vec<_>>();
        for address in keys.iter() {
            items.remove(address);
        }
        keys
    }
}
