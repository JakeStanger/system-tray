use crate::error::Result;
use std::collections::HashMap;
use std::ops::Deref;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

pub mod dbus_menu_proxy;
pub mod notifier_item_proxy;
pub mod notifier_watcher_proxy;
pub mod status_notifier_watcher;

/// Wrapper around map of properties fetched from a proxy.
pub(crate) struct DBusProps(pub HashMap<String, OwnedValue>);

impl DBusProps {
    /// Gets `key` from the map if present,
    /// downcasting it to type `T`.
    pub fn get<'a, T>(&'a self, key: &str) -> Option<Result<&'a T>>
    where
        T: ?Sized,
        &'a T: TryFrom<&'a Value<'a>>,
        <&'a T as TryFrom<&'a Value<'a>>>::Error: Into<zbus::zvariant::Error>,
    {
        self.0
            .get(key)
            .map(|v| v.downcast_ref().map_err(Into::into))
    }

    /// Gets `key` from the map if present,
    /// interpreting it as a `str`
    /// and converting it to a string.
    pub fn get_string(&self, key: &str) -> Option<Result<String>> {
        self.get::<str>(key).map(|res| res.map(ToString::to_string))
    }

    /// Gets `key` from the map if present,
    /// interpreting it as an `ObjectPath`,
    /// and converting it to a string.
    pub fn get_object_path(&self, key: &str) -> Option<Result<String>> {
        self.get::<ObjectPath>(key)
            .map(|res| res.map(ToString::to_string))
    }
}

pub(crate) trait OwnedValueExt {
    fn to_string(&self) -> Result<String>;
}

impl OwnedValueExt for OwnedValue {
    fn to_string(&self) -> Result<String> {
        self.downcast_ref::<&str>()
            .map(ToString::to_string)
            .map_err(Into::into)
    }
}

impl Deref for DBusProps {
    type Target = HashMap<String, OwnedValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
