/// # System Tray
///
/// An async implementation of the `StatusNotifierItem` and `DbusMenu` protocols for building system trays.
///
/// Requires Tokio.
///
/// ## Example
///
/// ```no_run
/// use system_tray::client::Client;
///
/// #[tokio::main]
/// async fn main() {
///     let client = Client::new().await.unwrap();
///     let mut tray_rx = client.subscribe();
///
///     let initial_items = client.items();
///
///     // do something with initial items...
///
///     while let Ok(ev) = tray_rx.recv().await {
///         println!("{ev:?}"); // do something with event...
///     }
/// }
/// ```
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

#[cfg(feature = "dbusmenu-gtk3")]
pub mod gtk_menu;

pub(crate) mod names {
    pub const WATCHER_BUS: &str = "org.kde.StatusNotifierWatcher";
    pub const WATCHER_OBJECT: &str = "/StatusNotifierWatcher";

    pub const ITEM_OBJECT: &str = "/StatusNotifierItem";
}
