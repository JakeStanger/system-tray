use crate::{item::StatusNotifierItem, menu::TrayMenu};
use std::sync::{Arc, Mutex};

#[cfg(feature = "data")]
pub type BaseMap = std::collections::HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>;

#[cfg(not(feature = "data"))]
type BaseMap = std::collections::HashSet<String>;

#[derive(Debug, Clone)]
pub(crate) struct TrayItemMap {
    inner: Arc<Mutex<BaseMap>>,
}

impl TrayItemMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Default::default())),
        }
    }

    #[cfg(feature = "data")]
    pub(crate) fn get_map(&self) -> Arc<Mutex<BaseMap>> {
        self.inner.clone()
    }

    pub(crate) fn new_item(&self, dest: String, item: &StatusNotifierItem) {
        let mut lock = self.inner.lock().expect("mutex lock should succeed");
        cfg_if::cfg_if! {
            if #[cfg(feature = "data")] {
                lock.insert(dest, (item.clone(), None));
            }else {
                let _ = item;
                lock.insert(dest);
            }
        }
    }

    pub(crate) fn remove_item(&self, dest: &str) {
        self.inner
            .lock()
            .expect("mutex lock should succeed")
            .remove(dest);
    }

    pub(crate) fn clear_items(&self) -> Vec<String> {
        let mut lock = self.inner.lock().expect("mutex lock should succeed");
        cfg_if::cfg_if! {
            if #[cfg(feature = "data")] {
                lock.drain().map(|(k, _)| k).collect()
            }else {
                lock.drain().collect()
            }
        }
    }

    pub(crate) fn update_menu(&self, dest: &str, menu: &TrayMenu) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "data")] {
                if let Some((_, menu_cache)) = self.inner.lock().unwrap().get_mut(dest) {
                    menu_cache.replace(menu.clone());
                } else {
                    tracing::error!("could not find item in state");
                }
            }else {
                let _ = menu;
                let _ = dest;
            }
        }
    }
}
