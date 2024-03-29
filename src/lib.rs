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
