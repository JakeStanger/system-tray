use crate::dbus::dbus_menu_proxy::{MenuLayout, PropertiesUpdate, UpdatedProps};
use crate::error::{Error, Result};
use serde::Deserialize;
use std::collections::HashMap;
use zbus::zvariant::{Array, OwnedValue, Structure, Value};

/// A menu that should be displayed when clicking corresponding tray icon
#[derive(Debug, Clone)]
pub struct TrayMenu {
    /// The unique identifier of the menu
    pub id: u32,
    /// A recursive list of submenus
    pub submenus: Vec<MenuItem>,
}

/// List of properties taken from:
/// <https://github.com/AyatanaIndicators/libdbusmenu/blob/4d03141aea4e2ad0f04ab73cf1d4f4bcc4a19f6c/libdbusmenu-glib/dbus-menu.xml#L75>
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MenuItem {
    /// Unique numeric id
    pub id: i32,

    /// Either a standard menu item or a separator [`MenuType`]
    pub menu_type: MenuType,
    /// Text of the item, except that:
    ///  - two consecutive underscore characters "__" are displayed as a
    ///    single underscore,
    ///  - any remaining underscore characters are not displayed at all,
    ///  - the first of those remaining underscore characters (unless it is
    ///    the last character in the string) indicates that the following
    ///    character is the access key.
    pub label: Option<String>,
    /// Whether the item can be activated or not.
    pub enabled: bool,
    /// True if the item is visible in the menu.
    pub visible: bool,
    /// Icon name of the item, following the freedesktop.org icon spec.
    pub icon_name: Option<String>,
    /// PNG data of the icon.
    pub icon_data: Option<Vec<u8>>,
    /// The shortcut of the item. Each array represents the key press
    /// in the list of keypresses. Each list of strings contains a list of
    /// modifiers and then the key that is used. The modifier strings
    /// allowed are: "Control", "Alt", "Shift" and "Super".
    ///
    /// - A simple shortcut like Ctrl+S is represented as:
    ///   [["Control", "S"]]
    /// - A complex shortcut like Ctrl+Q, Alt+X is represented as:
    ///   [["Control", "Q"], ["Alt", "X"]]
    pub shortcut: Option<Vec<Vec<String>>>,
    /// How the menuitem feels the information it's displaying to the
    /// user should be presented.
    /// See [`ToggleType`].
    pub toggle_type: ToggleType,
    /// Describe the current state of a "togglable" item.
    /// See [`ToggleState`].
    ///
    /// # Note:
    /// The implementation does not itself handle ensuring that only one
    /// item in a radio group is set to "on", or that a group does not have
    /// "on" and "indeterminate" items simultaneously; maintaining this
    /// policy is up to the toolkit wrappers.
    pub toggle_state: ToggleState,
    /// If the menu item has children this property should be set to
    /// "submenu".
    pub children_display: Option<String>,
    /// How the menuitem feels the information it's displaying to the
    /// user should be presented.
    /// See [`Disposition`]
    pub disposition: Disposition,
    /// Nested submenu items belonging to this item.
    pub submenu: Vec<MenuItem>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MenuDiff {
    pub id: i32,
    pub update: MenuItemUpdate,
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MenuItemUpdate {
    /// Text of the item, except that:
    ///  - two consecutive underscore characters "__" are displayed as a
    ///    single underscore,
    ///  - any remaining underscore characters are not displayed at all,
    ///  - the first of those remaining underscore characters (unless it is
    ///    the last character in the string) indicates that the following
    ///    character is the access key.
    pub label: Option<Option<String>>,
    /// Whether the item can be activated or not.
    pub enabled: Option<bool>,
    /// True if the item is visible in the menu.
    pub visible: Option<bool>,
    /// Icon name of the item, following the freedesktop.org icon spec.
    pub icon_name: Option<Option<String>>,
    /// PNG data of the icon.
    pub icon_data: Option<Option<Vec<u8>>>,
    /// Describe the current state of a "togglable" item.
    /// See [`ToggleState`].
    ///
    /// # Note:
    /// The implementation does not itself handle ensuring that only one
    /// item in a radio group is set to "on", or that a group does not have
    /// "on" and "indeterminate" items simultaneously; maintaining this
    /// policy is up to the toolkit wrappers.
    pub toggle_state: Option<ToggleState>,
    /// How the menuitem feels the information it's displaying to the
    /// user should be presented.
    /// See [`Disposition`]
    pub disposition: Option<Disposition>,
}

#[derive(Debug, Deserialize, Copy, Clone, Eq, PartialEq, Default)]
pub enum MenuType {
    ///  a separator
    Separator,
    /// an item which can be clicked to trigger an action or show another menu
    #[default]
    Standard,
}

impl From<&str> for MenuType {
    fn from(value: &str) -> Self {
        match value {
            "separator" => Self::Separator,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone, Eq, PartialEq, Default)]
pub enum ToggleType {
    /// Item is an independent togglable item
    Checkmark,
    /// Item is part of a group where only one item can be
    /// toggled at a time
    Radio,
    /// Item cannot be toggled
    #[default]
    CannotBeToggled,
}

impl From<&str> for ToggleType {
    fn from(value: &str) -> Self {
        match value {
            "checkmark" => Self::Checkmark,
            "radio" => Self::Radio,
            _ => Self::default(),
        }
    }
}

/// Describe the current state of a "togglable" item.
#[derive(Debug, Deserialize, Copy, Clone, Eq, PartialEq, Default)]
pub enum ToggleState {
    /// This item is toggled
    #[default]
    On,
    /// Item is not toggled
    Off,
    /// Item is not toggalble
    Indeterminate,
}

impl From<i32> for ToggleState {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::On,
            _ => Self::Indeterminate,
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone, Eq, PartialEq, Default)]
pub enum Disposition {
    /// a standard menu item
    #[default]
    Normal,
    /// providing additional information to the user
    Informative,
    ///  looking at potentially harmful results
    Warning,
    /// something bad could potentially happen
    Alert,
}

impl From<&str> for Disposition {
    fn from(value: &str) -> Self {
        match value {
            "informative" => Self::Informative,
            "warning" => Self::Warning,
            "alert" => Self::Alert,
            _ => Self::default(),
        }
    }
}

impl TryFrom<MenuLayout> for TrayMenu {
    type Error = Error;

    fn try_from(value: MenuLayout) -> Result<Self> {
        let submenus = value
            .fields
            .submenus
            .iter()
            .map(MenuItem::try_from)
            .collect::<std::result::Result<_, _>>()?;

        Ok(Self {
            id: value.id,
            submenus,
        })
    }
}

impl TryFrom<&OwnedValue> for MenuItem {
    type Error = Error;

    fn try_from(value: &OwnedValue) -> Result<Self> {
        let structure = value.downcast_ref::<&Structure>()?;

        let mut fields = structure.fields().iter();

        // defaults for enabled/visible are true
        // and setting here avoids having to provide a full `Default` impl
        let mut menu = MenuItem {
            enabled: true,
            visible: true,
            ..Default::default()
        };

        if let Some(Value::I32(id)) = fields.next() {
            menu.id = *id;
        }

        if let Some(Value::Dict(dict)) = fields.next() {
            menu.children_display = dict
                .get::<&str, &str>(&"children-display")?
                .map(str::to_string);

            // see: https://github.com/gnustep/libs-dbuskit/blob/4dc9b56216e46e0e385b976b0605b965509ebbbd/Bundles/DBusMenu/com.canonical.dbusmenu.xml#L76
            menu.label = dict
                .get::<&str, &str>(&"label")?
                .map(|label| label.replace('_', ""));

            if let Some(enabled) = dict.get::<&str, bool>(&"enabled")? {
                menu.enabled = enabled;
            }

            if let Some(visible) = dict.get::<&str, bool>(&"visible")? {
                menu.visible = visible;
            }

            menu.icon_name = dict.get::<&str, &str>(&"icon-name")?.map(str::to_string);

            if let Some(array) = dict.get::<&str, &Array>(&"icon-data")? {
                menu.icon_data = Some(get_icon_data(array)?);
            }

            if let Some(disposition) = dict
                .get::<&str, &str>(&"disposition")
                .ok()
                .flatten()
                .map(Disposition::from)
            {
                menu.disposition = disposition;
            }

            menu.toggle_state = dict
                .get::<&str, i32>(&"toggle-state")
                .ok()
                .flatten()
                .map(ToggleState::from)
                .unwrap_or_default();

            menu.toggle_type = dict
                .get::<&str, &str>(&"toggle-type")
                .ok()
                .flatten()
                .map(ToggleType::from)
                .unwrap_or_default();

            menu.menu_type = dict
                .get::<&str, &str>(&"type")
                .ok()
                .flatten()
                .map(MenuType::from)
                .unwrap_or_default();
        };

        if let Some(Value::Array(array)) = fields.next() {
            let mut submenu = vec![];
            for value in array.iter() {
                let value = OwnedValue::try_from(value)?;
                let menu = MenuItem::try_from(&value)?;
                submenu.push(menu);
            }

            menu.submenu = submenu;
        }

        Ok(menu)
    }
}

impl TryFrom<PropertiesUpdate<'_>> for Vec<MenuDiff> {
    type Error = Error;

    fn try_from(value: PropertiesUpdate<'_>) -> Result<Self> {
        let mut res = HashMap::new();

        for updated in value.updated {
            let id = updated.id;
            let update = MenuDiff {
                id,
                update: updated.try_into()?,
                ..Default::default()
            };

            res.insert(id, update);
        }

        for removed in value.removed {
            let update = res.entry(removed.id).or_insert_with(|| MenuDiff {
                id: removed.id,
                ..Default::default()
            });

            update.remove = removed.fields.iter().map(ToString::to_string).collect();
        }

        Ok(res.into_values().collect())
    }
}

impl TryFrom<UpdatedProps<'_>> for MenuItemUpdate {
    type Error = Error;

    fn try_from(value: UpdatedProps) -> Result<Self> {
        let dict = value.fields;

        let icon_data = if let Some(arr) = dict
            .get("icon-data")
            .map(Value::downcast_ref::<&Array>)
            .transpose()?
        {
            Some(Some(get_icon_data(arr)?))
        } else {
            None
        };

        Ok(Self {
            label: dict
                .get("label")
                .map(|v| v.downcast_ref::<&str>().map(ToString::to_string).ok()),

            enabled: dict
                .get("enabled")
                .and_then(|v| Value::downcast_ref::<bool>(v).ok()),

            visible: dict
                .get("visible")
                .and_then(|v| Value::downcast_ref::<bool>(v).ok()),

            icon_name: dict
                .get("icon-name")
                .map(|v| v.downcast_ref::<&str>().map(ToString::to_string).ok()),

            icon_data,

            toggle_state: dict
                .get("toggle-state")
                .and_then(|v| Value::downcast_ref::<i32>(v).ok())
                .map(ToggleState::from),

            disposition: dict
                .get("disposition")
                .and_then(|v| Value::downcast_ref::<&str>(v).ok())
                .map(Disposition::from),
        })
    }
}

fn get_icon_data(array: &Array) -> Result<Vec<u8>> {
    array
        .iter()
        .map(|v| v.downcast_ref::<u8>().map_err(Into::into))
        .collect::<Result<Vec<_>>>()
}
