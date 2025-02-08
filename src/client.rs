use crate::dbus::dbus_menu_proxy::{DBusMenuProxy, PropertiesUpdate};
use crate::dbus::notifier_item_proxy::StatusNotifierItemProxy;
use crate::dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use crate::dbus::status_notifier_watcher::StatusNotifierWatcher;
use crate::dbus::{self, OwnedValueExt};
use crate::error::{Error, Result};
use crate::item::{self, Status, StatusNotifierItem, Tooltip};
use crate::menu::{MenuDiff, TrayMenu};
use crate::names;
use dbus::DBusProps;
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::spawn;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, trace, warn};
use zbus::fdo::{DBusProxy, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::zvariant::{Structure, Value};
use zbus::{Connection, Message};

use self::names::ITEM_OBJECT;

/// An event emitted by the client
/// representing a change from either the `StatusNotifierItem`
/// or `DBusMenu` protocols.
#[derive(Debug, Clone)]
pub enum Event {
    /// A new `StatusNotifierItem` was added.
    Add(String, Box<StatusNotifierItem>),
    /// An update was received for an existing `StatusNotifierItem`.
    /// This could be either an update to the item itself,
    /// or an update to the associated menu.
    Update(String, UpdateEvent),
    /// A `StatusNotifierItem` was unregistered.
    Remove(String),
}

/// The specific change associated with an update event.
#[derive(Debug, Clone)]
pub enum UpdateEvent {
    AttentionIcon(Option<String>),
    Icon(Option<String>),
    OverlayIcon(Option<String>),
    Status(Status),
    Title(Option<String>),
    Tooltip(Option<Tooltip>),
    /// A menu layout has changed.
    /// The entire layout is sent.
    Menu(TrayMenu),
    /// One or more menu properties have changed.
    /// Only the updated properties are sent.
    MenuDiff(Vec<MenuDiff>),
    /// A new menu has connected to the item.
    /// Its name on bus is sent.
    MenuConnect(String),
}

/// A request to 'activate' one of the menu items,
/// typically sent when it is clicked.
#[derive(Debug, Clone)]
pub enum ActivateRequest {
    /// Submenu ID
    MenuItem {
        address: String,
        menu_path: String,
        submenu_id: i32,
    },
    /// Default activation for the tray.
    /// The parameter(x and y) represents screen coordinates and is to be considered an hint to the item where to show eventual windows (if any).
    Default { address: String, x: i32, y: i32 },
    /// Secondary activation(less important) for the tray.
    /// The parameter(x and y) represents screen coordinates and is to be considered an hint to the item where to show eventual windows (if any).
    Secondary { address: String, x: i32, y: i32 },
}

type State = HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>;

const PROPERTIES_INTERFACE: &str = "org.kde.StatusNotifierItem";

/// Client for watching the tray.
#[derive(Debug)]
pub struct Client {
    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,
    connection: Connection,

    items: Arc<Mutex<State>>,
}

impl Client {
    /// Creates and initializes the client.
    ///
    /// The client will begin listening to items and menus and sending events immediately.
    /// It is recommended that consumers immediately follow the call to `new` with a `subscribe` call,
    /// then immediately follow that with a call to `items` to get the state to not miss any events.
    ///
    /// The value of `service_name` must be unique on the session bus.
    /// It is recommended to use something similar to the format of `appid-numid`,
    /// where `numid` is a short-ish random integer.
    ///
    /// # Errors
    ///
    /// If the initialization fails for any reason,
    /// for example if unable to connect to the bus,
    /// this method will return an error.
    ///
    /// # Panics
    ///
    /// If the generated well-known name is invalid, the library will panic
    /// as this indicates a major bug.
    ///
    /// Likewise, the spawned tasks may panic if they cannot get a `Mutex` lock.
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        let (tx, rx) = broadcast::channel(32);

        // first start server...
        StatusNotifierWatcher::new().attach_to(&connection).await?;

        // ...then connect to it
        let watcher_proxy = StatusNotifierWatcherProxy::new(&connection).await?;

        // register a host on the watcher to declare we want to watch items
        // get a well-known name
        let pid = std::process::id();
        let mut i = 0;
        let wellknown = loop {
            use zbus::fdo::RequestNameReply::*;

            i += 1;
            let wellknown = format!("org.freedesktop.StatusNotifierHost-{pid}-{i}");
            let wellknown: zbus::names::WellKnownName = wellknown
                .try_into()
                .expect("generated well-known name is invalid");

            let flags = [zbus::fdo::RequestNameFlags::DoNotQueue];
            match connection
                .request_name_with_flags(&wellknown, flags.into_iter().collect())
                .await?
            {
                PrimaryOwner => break wellknown,
                Exists | AlreadyOwner => {}
                InQueue => unreachable!(
                    "request_name_with_flags returned InQueue even though we specified DoNotQueue"
                ),
            };
        };

        debug!("wellknown: {wellknown}");
        watcher_proxy
            .register_status_notifier_host(&wellknown)
            .await?;

        let items = Arc::new(Mutex::new(HashMap::new()));

        // handle new items
        {
            let connection = connection.clone();
            let tx = tx.clone();
            let items = items.clone();

            let mut stream = watcher_proxy
                .receive_status_notifier_item_registered()
                .await?;

            spawn(async move {
                while let Some(item) = stream.next().await {
                    let address = item.args().map(|args| args.service);

                    if let Ok(address) = address {
                        debug!("received new item: {address}");
                        if let Err(err) = Self::handle_item(
                            address,
                            connection.clone(),
                            tx.clone(),
                            items.clone(),
                        )
                        .await
                        {
                            error!("{err}");
                            break;
                        }
                    }
                }

                Ok::<(), Error>(())
            });
        }

        // then lastly get all items
        // it can take so long to fetch all items that we have to do this last,
        // otherwise some incoming items get missed
        {
            let connection = connection.clone();
            let tx = tx.clone();
            let items = items.clone();

            spawn(async move {
                let initial_items = watcher_proxy.registered_status_notifier_items().await?;
                debug!("initial items: {initial_items:?}");

                for item in initial_items {
                    if let Err(err) =
                        Self::handle_item(&item, connection.clone(), tx.clone(), items.clone())
                            .await
                    {
                        error!("{err}");
                    }
                }

                Ok::<(), Error>(())
            });
        }

        // Handle other watchers unregistering and this one taking over
        // It is necessary to clear all items as our watcher will then re-send them all
        {
            let tx = tx.clone();
            let items = items.clone();

            let dbus_proxy = DBusProxy::new(&connection).await?;

            let mut stream = dbus_proxy.receive_name_acquired().await?;

            spawn(async move {
                while let Some(thing) = stream.next().await {
                    let body = thing.args()?;
                    if body.name == names::WATCHER_BUS {
                        let mut items = items.lock().expect("mutex lock should succeed");
                        let keys = items.keys().cloned().collect::<Vec<_>>();
                        for address in keys {
                            items.remove(&address);
                            tx.send(Event::Remove(address))?;
                        }
                    }
                }

                Ok::<(), Error>(())
            });
        }

        debug!("tray client initialized");

        Ok(Self {
            connection,
            tx,
            _rx: rx,
            items,
        })
    }

    /// Processes an incoming item to send the initial add event,
    /// then set up listeners for it and its menu.
    async fn handle_item(
        address: &str,
        connection: Connection,
        tx: broadcast::Sender<Event>,
        items: Arc<Mutex<State>>,
    ) -> crate::error::Result<()> {
        let (destination, path) = parse_address(address);

        let properties_proxy = PropertiesProxy::builder(&connection)
            .destination(destination.to_string())?
            .path(path.clone())?
            .build()
            .await?;

        let properties = Self::get_item_properties(destination, &path, &properties_proxy).await?;

        items
            .lock()
            .expect("mutex lock should succeed")
            .insert(destination.into(), (properties.clone(), None));

        tx.send(Event::Add(
            destination.to_string(),
            properties.clone().into(),
        ))?;

        {
            let connection = connection.clone();
            let destination = destination.to_string();
            let items = items.clone();
            let tx = tx.clone();

            spawn(async move {
                Self::watch_item_properties(
                    &destination,
                    &path,
                    &connection,
                    properties_proxy,
                    items,
                    tx,
                )
                .await?;

                debug!("Stopped watching {destination}{path}");
                Ok::<(), Error>(())
            });
        }

        if let Some(menu) = properties.menu {
            let destination = destination.to_string();

            tx.send(Event::Update(
                destination.clone(),
                UpdateEvent::MenuConnect(menu.clone()),
            ))?;

            spawn(async move {
                Self::watch_menu(destination, &menu, &connection, tx, items).await?;
                Ok::<(), Error>(())
            });
        }

        Ok(())
    }

    /// Gets the properties for an SNI item.
    async fn get_item_properties(
        destination: &str,
        path: &str,
        properties_proxy: &PropertiesProxy<'_>,
    ) -> crate::error::Result<StatusNotifierItem> {
        let properties = properties_proxy
            .get_all(
                InterfaceName::from_static_str(PROPERTIES_INTERFACE)
                    .expect("to be valid interface name"),
            )
            .await;

        let properties = match properties {
            Ok(properties) => properties,
            Err(err) => {
                error!("Error fetching properties from {destination}{path}: {err:?}");
                return Err(err.into());
            }
        };

        StatusNotifierItem::try_from(DBusProps(properties))
    }

    /// Watches an SNI item's properties,
    /// sending an update event whenever they change.
    async fn watch_item_properties(
        destination: &str,
        path: &str,
        connection: &Connection,
        properties_proxy: PropertiesProxy<'_>,
        items: Arc<Mutex<State>>,
        tx: broadcast::Sender<Event>,
    ) -> crate::error::Result<()> {
        let notifier_item_proxy = StatusNotifierItemProxy::builder(connection)
            .destination(destination)?
            .path(path)?
            .build()
            .await?;

        let dbus_proxy = DBusProxy::new(connection).await?;

        let mut disconnect_stream = dbus_proxy.receive_name_owner_changed().await?;
        let mut props_changed = notifier_item_proxy.inner().receive_all_signals().await?;

        loop {
            tokio::select! {
                Some(change) = props_changed.next() => {
                    match Self::get_update_event(change, &properties_proxy).await {
                        Ok(Some(event)) => {
                                debug!("[{destination}{path}] received property change: {event:?}");
                                tx.send(Event::Update(destination.to_string(), event))?;
                            }
                        Err(e) => {
                            error!("Error parsing update properties from {destination}{path}: {e:?}");
                        }
                        _ => {}
                    }
                }
                Some(signal) = disconnect_stream.next() => {
                    let args = signal.args()?;
                    let old = args.old_owner();
                    let new = args.new_owner();

                    if let (Some(old), None) = (old.as_ref(), new.as_ref()) {
                        if old == destination {
                            debug!("[{destination}{path}] disconnected");

                            let watcher_proxy = StatusNotifierWatcherProxy::new(connection)
                                .await
                                .expect("Failed to open StatusNotifierWatcherProxy");

                            if let Err(error) = watcher_proxy.unregister_status_notifier_item(old).await {
                                error!("{error:?}");
                            }

                            items.lock().expect("mutex lock should succeed").remove(&destination.to_string());

                            tx.send(Event::Remove(destination.to_string()))?;
                            break Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Gets the update event for a `DBus` properties change message.
    async fn get_update_event(
        change: Message,
        properties_proxy: &PropertiesProxy<'_>,
    ) -> Result<Option<UpdateEvent>> {
        let header = change.header();
        let member = header
            .member()
            .ok_or(Error::InvalidData("Update message header missing `member`"))?;

        let property_name = match member.as_str() {
            "NewAttentionIcon" => "AttentionIconName",
            "NewIcon" => "IconName",
            "NewOverlayIcon" => "OverlayIconName",
            "NewStatus" => "Status",
            "NewTitle" => "Title",
            "NewToolTip" => "ToolTip",
            _ => &member.as_str()["New".len()..],
        };

        let property = properties_proxy
            .get(
                InterfaceName::from_static_str(PROPERTIES_INTERFACE)
                    .expect("to be valid interface name"),
                property_name,
            )
            .await?;

        debug!("received tray item update: {member} -> {property:?}");

        use UpdateEvent::*;
        Ok(match member.as_str() {
            "NewAttentionIcon" => Some(AttentionIcon(property.to_string().ok())),
            "NewIcon" => Some(Icon(property.to_string().ok())),
            "NewOverlayIcon" => Some(OverlayIcon(property.to_string().ok())),
            "NewStatus" => Some(Status(
                property.downcast_ref::<&str>().map(item::Status::from)?,
            )),
            "NewTitle" => Some(Title(property.to_string().ok())),
            "NewToolTip" => Some(Tooltip({
                property
                    .downcast_ref::<&Structure>()
                    .ok()
                    .map(crate::item::Tooltip::try_from)
                    .transpose()?
            })),
            _ => {
                warn!("received unhandled update event: {member}");
                None
            }
        })
    }

    /// Watches the `DBusMenu` associated with an SNI item.
    ///
    /// This gets the initial menu, sending an update event immediately.
    /// Update events are then sent for any further updates
    /// until the item is removed.
    async fn watch_menu(
        destination: String,
        menu_path: &str,
        connection: &Connection,
        tx: broadcast::Sender<Event>,
        items: Arc<Mutex<State>>,
    ) -> crate::error::Result<()> {
        let dbus_menu_proxy = DBusMenuProxy::builder(connection)
            .destination(destination.as_str())?
            .path(menu_path)?
            .build()
            .await?;

        let menu = dbus_menu_proxy.get_layout(0, 10, &[]).await?;
        let menu = TrayMenu::try_from(menu)?;

        if let Some((_, menu_cache)) = items
            .lock()
            .expect("mutex lock should succeed")
            .get_mut(&destination)
        {
            menu_cache.replace(menu.clone());
        } else {
            error!("could not find item in state");
        }

        tx.send(Event::Update(
            destination.to_string(),
            UpdateEvent::Menu(menu),
        ))?;

        let mut layout_updated = dbus_menu_proxy.receive_layout_updated().await?;
        let mut properties_updated = dbus_menu_proxy.receive_items_properties_updated().await?;

        loop {
            tokio::select!(
                Some(_) = layout_updated.next() => {
                    debug!("[{destination}{menu_path}] layout update");

                    let get_layout = dbus_menu_proxy.get_layout(0, 10, &[]);

                    let menu = match timeout(Duration::from_secs(1), get_layout).await {
                        Ok(Ok(menu)) => {
                            debug!("got new menu layout");
                            menu
                        }
                        Ok(Err(err)) => {
                            error!("error fetching layout: {err:?}");
                            break;
                        }
                        Err(_) => {
                            error!("Timeout getting layout");
                            break;
                        }
                    };

                    let menu = TrayMenu::try_from(menu)?;

                    if let Some((_, menu_cache)) = items
                        .lock()
                        .expect("mutex lock should succeed")
                        .get_mut(&destination)
                    {
                        menu_cache.replace(menu.clone());
                    } else {
                        error!("could not find item in state");
                    }

                    debug!("sending new menu for '{destination}'");
                    trace!("new menu for '{destination}': {menu:?}");
                    tx.send(Event::Update(
                        destination.to_string(),
                        UpdateEvent::Menu(menu),
                    ))?;
                }
                Some(change) = properties_updated.next() => {
                    let body = change.message().body();
                    let update: PropertiesUpdate= body.deserialize::<PropertiesUpdate>()?;
                    let diffs = Vec::try_from(update)?;

                    tx.send(Event::Update(
                        destination.to_string(),
                        UpdateEvent::MenuDiff(diffs),
                    ))?;

                    // FIXME: Menu cache gonna be out of sync
                }
            );
        }

        Ok(())
    }

    async fn get_notifier_item_proxy(
        &self,
        address: String,
    ) -> crate::error::Result<StatusNotifierItemProxy<'_>> {
        let proxy = StatusNotifierItemProxy::builder(&self.connection)
            .destination(address)?
            .path(ITEM_OBJECT)?
            .build()
            .await?;
        Ok(proxy)
    }

    async fn get_menu_proxy(
        &self,
        address: String,
        menu_path: String,
    ) -> crate::error::Result<DBusMenuProxy<'_>> {
        let proxy = DBusMenuProxy::builder(&self.connection)
            .destination(address)?
            .path(menu_path)?
            .build()
            .await?;
        Ok(proxy)
    }

    /// Subscribes to the events broadcast channel,
    /// returning a new receiver.
    ///
    /// Once the client is dropped, the receiver will close.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Gets all current items, including their menus if present.
    #[must_use]
    pub fn items(&self) -> Arc<Mutex<State>> {
        self.items.clone()
    }

    /// One should call this method with id=0 when opening the root menu.
    ///
    /// ID refers to the menuitem id.
    /// Returns `needsUpdate`
    pub async fn about_to_show_menuitem(
        &self,
        address: String,
        menu_path: String,
        id: i32,
    ) -> crate::error::Result<bool> {
        let proxy = self.get_menu_proxy(address, menu_path).await?;
        Ok(proxy.about_to_show(id).await?)
    }

    /// Sends an activate request for a menu item.
    ///
    /// # Errors
    ///
    /// The method will return an error if the connection to the `DBus` object fails,
    /// or if sending the event fails for any reason.
    ///
    /// # Panics
    ///
    /// If the system time is somehow before the Unix epoch.
    pub async fn activate(&self, req: ActivateRequest) -> crate::error::Result<()> {
        macro_rules! timeout_event {
            ($event:expr) => {
                if timeout(Duration::from_secs(1), $event).await.is_err() {
                    error!("Timed out sending activate event");
                }
            };
        }
        match req {
            ActivateRequest::MenuItem {
                address,
                menu_path,
                submenu_id,
            } => {
                let proxy = self.get_menu_proxy(address, menu_path).await?;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time should flow forwards");

                let event = proxy.event(
                    submenu_id,
                    "clicked",
                    &Value::I32(0),
                    timestamp.as_secs() as u32,
                );

                timeout_event!(event);
            }
            ActivateRequest::Default { address, x, y } => {
                let proxy = self.get_notifier_item_proxy(address).await?;
                let event = proxy.activate(x, y);

                timeout_event!(event);
            }
            ActivateRequest::Secondary { address, x, y } => {
                let proxy = self.get_notifier_item_proxy(address).await?;
                let event = proxy.secondary_activate(x, y);

                timeout_event!(event);
            }
        }

        Ok(())
    }
}

fn parse_address(address: &str) -> (&str, String) {
    address
        .split_once('/')
        .map_or((address, String::from("/StatusNotifierItem")), |(d, p)| {
            (d, format!("/{p}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unnamed() {
        let address = ":1.58/StatusNotifierItem";
        let (destination, path) = parse_address(address);

        assert_eq!(":1.58", destination);
        assert_eq!("/StatusNotifierItem", path);
    }

    #[test]
    fn parse_named() {
        let address = ":1.72/org/ayatana/NotificationItem/dropbox_client_1398";
        let (destination, path) = parse_address(address);

        assert_eq!(":1.72", destination);
        assert_eq!("/org/ayatana/NotificationItem/dropbox_client_1398", path);
    }
}
