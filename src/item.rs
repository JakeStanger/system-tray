use crate::dbus::DBusProps;
use crate::error::Error;
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use zbus::zvariant::{Array, Structure};

/// Represents an item to display inside the tray.
/// <https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/>
#[derive(Deserialize, Debug, Clone)]
pub struct StatusNotifierItem {
    /// A name that should be unique for this application and consistent between sessions, such as the application name itself.
    pub id: String,

    /// The category of this item.
    ///
    /// The allowed values for the Category property are:
    ///
    /// - `ApplicationStatus`: The item describes the status of a generic application, for instance the current state of a media player.
    ///     In the case where the category of the item can not be known, such as when the item is being proxied from another incompatible or emulated system,
    ///     `ApplicationStatus` can be used a sensible default fallback.
    /// - `Communications`: The item describes the status of communication oriented applications, like an instant messenger or an email client.
    /// - `SystemServices`: The item describes services of the system not seen as a stand alone application by the user, such as an indicator for the activity of a disk indexing service.
    /// - `Hardware`: The item describes the state and control of a particular hardware, such as an indicator of the battery charge or sound card volume control.
    pub category: Category,

    /// A name that describes the application, it can be more descriptive than Id.
    pub title: Option<String>,

    /// Describes the status of this item or of the associated application.
    ///
    /// The allowed values for the Status property are:
    ///
    /// - Passive: The item doesn't convey important information to the user, it can be considered an "idle" status and is likely that visualizations will chose to hide it.
    /// - Active: The item is active, is more important that the item will be shown in some way to the user.
    /// - NeedsAttention: The item carries really important information for the user, such as battery charge running out and is wants to incentive the direct user intervention.
    ///     Visualizations should emphasize in some way the items with NeedsAttention status.
    pub status: Status,

    /// The windowing-system dependent identifier for a window, the application can choose one of its windows to be available through this property or just set 0 if it's not interested.
    pub window_id: u32,

    pub icon_theme_path: Option<String>,

    /// The StatusNotifierItem can carry an icon that can be used by the visualization to identify the item.
    ///
    /// An icon can either be identified by its Freedesktop-compliant icon name, carried by this property of by the icon data itself, carried by the property `IconPixmap`.
    /// Visualizations are encouraged to prefer icon names over icon pixmaps if both are available
    /// (still not very defined: could be the pixmap used as fallback if an icon name is not found?)
    pub icon_name: Option<String>,

    /// Carries an ARGB32 binary representation of the icon, the format of icon data used in this specification is described in Section Icons
    ///
    /// # Icons
    ///
    /// All the icons can be transferred over the bus by a particular serialization of their data,
    /// capable of representing multiple resolutions of the same image or a brief aimation of images of the same size.
    ///
    /// Icons are transferred in an array of raw image data structures of signature a(iiay) whith each one describing the width, height, and image data respectively.
    /// The data is represented in ARGB32 format and is in the network byte order, to make easy the communication over the network between little and big endian machines.
    pub icon_pixmap: Option<Vec<IconPixmap>>,

    /// The Freedesktop-compliant name of an icon.
    /// This can be used by the visualization to indicate extra state information, for instance as an overlay for the main icon.
    pub overlay_icon_name: Option<String>,

    /// ARGB32 binary representation of the overlay icon described in the previous paragraph.
    pub overlay_icon_pixmap: Option<Vec<IconPixmap>>,

    /// The Freedesktop-compliant name of an icon. this can be used by the visualization to indicate that the item is in RequestingAttention state.
    pub attention_icon_name: Option<String>,

    /// ARGB32 binary representation of the requesting attention icon describe in the previous paragraph.
    pub attention_icon_pixmap: Option<Vec<IconPixmap>>,

    /// An item can also specify an animation associated to the RequestingAttention state.
    /// This should be either a Freedesktop-compliant icon name or a full path.
    /// The visualization can choose between the movie or AttentionIconPixmap (or using neither of those) at its discretion.
    pub attention_movie_name: Option<String>,

    // /// Data structure that describes extra information associated to this item, that can be visualized for instance by a tooltip
    // /// (or by any other mean the visualization consider appropriate.
    // pub tool_tip: Option<Tooltip>,
    /// The item only support the context menu, the visualization should prefer showing the menu or sending ContextMenu() instead of Activate()
    pub item_is_menu: bool,

    /// DBus path to an object which should implement the com.canonical.dbusmenu interface
    pub menu: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum Category {
    #[default]
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

impl From<&str> for Category {
    fn from(value: &str) -> Self {
        match value {
            "Communications" => Self::Communications,
            "SystemServices" => Self::SystemServices,
            "Hardware" => Self::Hardware,
            _ => Self::ApplicationStatus,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum Status {
    #[default]
    Unknown,
    Passive,
    Active,
    NeedsAttention,
}

impl From<&str> for Status {
    fn from(value: &str) -> Self {
        match value {
            "Passive" => Self::Passive,
            "Active" => Self::Active,
            "NeedsAttention" => Self::NeedsAttention,
            _ => Self::Unknown,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl Debug for IconPixmap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconPixmap")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixels", &format!("<length: {}>", self.pixels.len()))
            .finish()
    }
}

impl IconPixmap {
    // TODO: rewrite
    fn from_array(array: &Array) -> Option<Vec<Self>> {
        let mut pixmaps = vec![];

        array.iter().for_each(|b| {
            let s = b.downcast_ref::<Structure>();
            let fields = s.unwrap().fields();
            let width = fields[0].downcast_ref::<i32>().unwrap();
            let height = fields[1].downcast_ref::<i32>().unwrap();
            let pixel_values = fields[2].downcast_ref::<Array>().unwrap().get();
            let mut pixels = vec![];
            pixel_values.iter().for_each(|p| {
                pixels.push(*p.downcast_ref::<u8>().unwrap());
            });
            pixmaps.push(IconPixmap {
                width: *width,
                height: *height,
                pixels,
            })
        });

        Some(pixmaps)
    }
}

// /// Data structure that describes extra information associated to this item, that can be visualized for instance by a tooltip
// /// (or by any other mean the visualization consider appropriate.
// #[derive(Debug, Clone, Deserialize)]
// pub struct Tooltip {
//     // TODO
// }
//
// impl From<&Structure<'_>> for Tooltip {
//     fn from(value: &Structure) -> Self {
//         todo!()
//     }
// }

impl TryFrom<DBusProps> for StatusNotifierItem {
    type Error = Error;

    fn try_from(props: DBusProps) -> Result<Self, Self::Error> {
        if let Some(id) = props.get_string("Id") {
            Ok(Self {
                id,
                title: props.get_string("Title"),
                status: props.get_status(),
                window_id: props.get::<u32>("WindowId").copied().unwrap_or_default(),
                icon_theme_path: props.get_string("IconThemePath"),
                icon_name: props.get_string("IconName"),
                icon_pixmap: props.get_icon_pixmap("IconPixmap"),
                overlay_icon_name: props.get_string("OverlayIconName"),
                overlay_icon_pixmap: props.get_icon_pixmap("OverlayIconPixmap"),
                attention_icon_name: props.get_string("AttentionIconName"),
                attention_icon_pixmap: props.get_icon_pixmap("AttentionIconPixmap"),
                attention_movie_name: props.get_string("AttentionMovieName"),
                // tool_tip: props.get_tooltip(),
                item_is_menu: props.get("ItemIsMenu").copied().unwrap_or_default(),
                category: props.get_category(),
                menu: props.get_object_path("Menu"),
            })
        } else {
            Err(Error::MissingProperty("Id"))
        }
    }
}

impl DBusProps {
    fn get_category(&self) -> Category {
        self.get::<str>("Category")
            .map(Category::from)
            .unwrap_or_default()
    }

    fn get_status(&self) -> Status {
        self.get::<str>("Status")
            .map(Status::from)
            .unwrap_or_default()
    }

    fn get_icon_pixmap(&self, key: &str) -> Option<Vec<IconPixmap>> {
        self.get::<Array>(key)
            .map(|arr| IconPixmap::from_array(arr))
            .unwrap_or(None)
    }

    // fn get_tooltip(&self) -> Option<Tooltip> {
    //     self.get::<Structure>("Tooltip")
    //         .map(Tooltip::from)
    // }
}
