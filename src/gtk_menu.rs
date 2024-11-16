/// NOTE: This file is actually copied and amended
/// from the `dbusmenu-gtk3` crate.
use dbusmenu_gtk3_sys as ffi;
use glib::translate::*;
use gtk::glib;
use std::fmt;

glib::wrapper! {
    #[doc(alias = "DbusmenuGtkMenu")]
    pub struct Menu(Object<ffi::DbusmenuGtkMenu, ffi::DbusmenuGtkMenuClass>) @extends gtk::Menu, gtk::MenuShell, gtk::Container, gtk::Widget, glib::object::InitiallyUnowned, @implements gtk::Buildable;

    match fn {
        type_ => || ffi::dbusmenu_gtkmenu_get_type(),
    }
}

impl Menu {
    pub const NONE: Option<&'static Menu> = None;

    /// Creates a new [`Menu`][crate::Menu] object and creates a [`dbusmenu_glib::Client`][crate::dbusmenu_glib::Client]
    /// that connects across `DBus` to a `DbusmenuServer`.
    /// ## `dbus_name`
    /// Name of the `DbusmenuServer` on `DBus`
    /// ## `dbus_object`
    /// Name of the object on the `DbusmenuServer`
    ///
    /// # Returns
    ///
    /// A new [`Menu`][crate::Menu] sync'd with a server
    #[doc(alias = "dbusmenu_gtkmenu_new")]
    #[must_use]
    pub fn new(dbus_name: &str, dbus_object: &str) -> Menu {
        unsafe {
            from_glib_none(ffi::dbusmenu_gtkmenu_new(
                dbus_name.to_glib_none().0,
                dbus_object.to_glib_none().0,
            ))
        }
    }
}

impl fmt::Display for Menu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Menu")
    }
}
