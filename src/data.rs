use crate::{
    item::StatusNotifierItem,
    menu::{MenuDiff, MenuItem, MenuItemUpdate, TrayMenu},
};
use std::sync::{Arc, Mutex};

#[cfg(feature = "data")]
use {crate::client::UpdateEvent, tracing::error};

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
            inner: Arc::new(Mutex::new(BaseMap::default())),
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
                if let Some((_, menu_cache)) = self.inner
                        .lock()
                        .expect("should get lock")
                        .get_mut(dest) {
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

    #[cfg(feature = "data")]
    pub(crate) fn apply_update_event(&self, dest: &str, event: &UpdateEvent) {
        if let Some((item, menu)) = self
            .inner
            .lock()
            .expect("mutex lock should succeed")
            .get_mut(dest)
        {
            match event {
                UpdateEvent::AttentionIcon(icon_name) => {
                    item.attention_icon_name.clone_from(icon_name);
                }
                UpdateEvent::Icon {
                    icon_name,
                    icon_pixmap,
                } => {
                    item.icon_name.clone_from(icon_name);
                    item.icon_pixmap = if icon_pixmap.is_empty() {
                        None
                    } else {
                        Some(icon_pixmap.clone())
                    }
                }
                UpdateEvent::OverlayIcon(icon_name) => item.overlay_icon_name.clone_from(icon_name),
                UpdateEvent::Status(status) => item.status = *status,
                UpdateEvent::Title(title) => item.title.clone_from(title),
                UpdateEvent::Tooltip(tooltip) => item.tool_tip.clone_from(tooltip),
                UpdateEvent::Menu(tray_menu) => *menu = Some(tray_menu.clone()),
                UpdateEvent::MenuConnect(menu) => item.menu = Some(menu.clone()),
                UpdateEvent::MenuDiff(menu_diffs) => {
                    if let Some(menu) = menu {
                        apply_menu_diffs(menu, menu_diffs);
                    }
                }
            }
        } else {
            error!("could not find item in state");
        }
    }
}

pub fn apply_menu_diffs(tray_menu: &mut TrayMenu, diffs: &[MenuDiff]) {
    let mut diff_iter = diffs.iter().peekable();
    tray_menu.submenus.iter_mut().for_each(|item| {
        if let Some(diff) = diff_iter.next_if(|d| d.id == item.id) {
            apply_menu_item_diff(item, &diff.update);
        }
    });
}

fn apply_menu_item_diff(menu_item: &mut MenuItem, update: &MenuItemUpdate) {
    if let Some(label) = &update.label {
        menu_item.label.clone_from(label);
    }
    if let Some(enabled) = update.enabled {
        menu_item.enabled = enabled;
    }
    if let Some(visible) = update.visible {
        menu_item.visible = visible;
    }
    if let Some(icon_name) = &update.icon_name {
        menu_item.icon_name.clone_from(icon_name);
    }
    if let Some(icon_data) = &update.icon_data {
        menu_item.icon_data.clone_from(icon_data);
    }
    if let Some(toggle_state) = update.toggle_state {
        menu_item.toggle_state = toggle_state;
    }
    if let Some(disposition) = update.disposition {
        menu_item.disposition = disposition;
    }
}
