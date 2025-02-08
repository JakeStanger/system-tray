#[cfg(feature = "data")]
pub use data_all::TrayItemMap;

#[cfg(not(feature = "data"))]
pub use data_destination_only::TrayItemMap;

#[cfg(feature = "data")]
mod data_all {
    use crate::{item::StatusNotifierItem, menu::TrayMenu};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use tracing::error;

    type BaseMap = HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>;

    #[derive(Debug, Clone)]
    pub struct TrayItemMap {
        inner: Arc<Mutex<BaseMap>>,
    }
    impl TrayItemMap {
        pub fn get_map(&self) -> Arc<Mutex<BaseMap>> {
            self.inner.clone()
        }

        pub(crate) fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(HashMap::new())),
            }
        }
        pub(crate) fn new_item(&self, dest: String, item: &StatusNotifierItem) {
            self.inner
                .lock()
                .expect("mutex lock should succeed")
                .insert(dest, (item.clone(), None));
        }
        pub(crate) fn remove_item(&self, dest: &str) {
            self.inner
                .lock()
                .expect("mutex lock should succeed")
                .remove(dest);
        }
        pub(crate) fn update_menu(&self, dest: &str, menu: &TrayMenu) {
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
        pub(crate) fn clear_items(&self) -> Vec<String> {
            let mut items = self.inner.lock().expect("mutex lock should succeed");
            items.drain().map(|(k, _)| k).collect()
        }
    }
}

// #[cfg(not(feature = "data"))]
mod data_destination_only {
    use crate::{item::StatusNotifierItem, menu::TrayMenu};
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    #[derive(Debug, Clone)]
    pub struct TrayItemMap {
        inner: Arc<Mutex<HashSet<String>>>,
    }
    impl TrayItemMap {
        pub(crate) fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(HashSet::new())),
            }
        }
        pub(crate) fn new_item(&self, dest: String, _: &StatusNotifierItem) {
            self.inner
                .lock()
                .expect("mutex lock should succeed")
                .insert(dest);
        }
        pub(crate) fn remove_item(&self, dest: &str) {
            self.inner
                .lock()
                .expect("mutex lock should succeed")
                .remove(dest);
        }
        pub(crate) fn update_menu(&self, _: &str, _: &TrayMenu) {
            // WE DO NOTHING HERE
        }
        pub(crate) fn clear_items(&self) -> Vec<String> {
            let mut items = self.inner.lock().expect("mutex lock should succeed");
            items.drain().collect()
        }
    }
}
