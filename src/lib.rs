#![doc = include_str!("../README.md")]

mod dbus;

/// Client for listening to item and menu events,
/// and associated types.
pub mod client;

/// Error and result types.
pub mod error;

/// `StatusNotifierItem` item representation.
pub mod item;

/// `DBusMenu` menu representation.
pub mod menu;

pub(crate) mod names {
    pub const WATCHER_BUS: &str = "org.kde.StatusNotifierWatcher";
    pub const WATCHER_OBJECT: &str = "/StatusNotifierWatcher";

    pub const ITEM_OBJECT: &str = "/StatusNotifierItem";
}