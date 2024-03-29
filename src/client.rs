use crate::dbus::dbus_menu_proxy::DBusMenuProxy;
use crate::dbus::notifier_item_proxy::StatusNotifierItemProxy;
use crate::dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use crate::dbus::status_notifier_watcher::StatusNotifierWatcher;
use crate::dbus::{self, OwnedValueExt};
use crate::error::Error;
use crate::item::{self, Status, StatusNotifierItem};
use crate::menu::TrayMenu;
use dbus::DBusProps;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::spawn;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};
use zbus::export::ordered_stream::OrderedStreamExt;
use zbus::fdo::{DBusProxy, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::zvariant::Value;
use zbus::{Connection, ConnectionBuilder, Message};

/// An event emitted by the client
/// representing a change from either the StatusNotifierItem
/// or DBusMenu protocols.
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
    // Tooltip(Option<Tooltip>),
    Menu(TrayMenu),
}

/// A request to 'activate' one of the menu items,
/// typically sent when it is clicked.
#[derive(Debug, Clone)]
pub struct ActivateRequest {
    pub address: String,
    pub menu_path: String,
    pub submenu_id: i32,
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
    pub async fn new(service_name: &str) -> crate::error::Result<Self> {
        let (tx, rx) = broadcast::channel(32);

        // first start server...
        let watcher = StatusNotifierWatcher::new();

        let connection = ConnectionBuilder::session()?
            .name("org.kde.StatusNotifierWatcher")?
            .serve_at("/StatusNotifierWatcher", watcher)?
            .build()
            .await?;

        // ...then connect to it
        let watcher_proxy = StatusNotifierWatcherProxy::new(&connection).await?;

        // register a host on the watcher to declare we want to watch items
        let service_name = format!("StatusNotifierHost-{service_name}");
        watcher_proxy
            .register_status_notifier_host(&service_name)
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
                        Self::handle_item(address, connection.clone(), tx.clone(), items.clone())
                            .await?;
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
                    Self::handle_item(&item, connection.clone(), tx.clone(), items.clone()).await?;
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
        let (destination, path) = address
            .split_once('/')
            .map_or((address, Cow::Borrowed("/StatusNotifierItem")), |(d, p)| {
                (d, Cow::Owned(format!("/{p}")))
            });

        let properties_proxy = PropertiesProxy::builder(&connection)
            .destination(destination.to_string())?
            .path(path.clone())?
            .build()
            .await?;

        let properties = Self::get_item_properties(destination, &path, &properties_proxy).await?;

        items
            .lock()
            .expect("to get lock")
            .insert(destination.into(), (properties.clone(), None));

        tx.send(Event::Add(
            destination.to_string(),
            properties.clone().into(),
        ))?;

        {
            let connection = connection.clone();
            let destination = destination.to_string();
            let tx = tx.clone();

            spawn(async move {
                Self::watch_item_properties(&destination, &path, &connection, properties_proxy, tx)
                    .await?;

                debug!("Stopped watching {destination}{path}");
                Ok::<(), Error>(())
            });
        }

        if let Some(menu) = properties.menu {
            let destination = destination.to_string();
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
        tx: broadcast::Sender<Event>,
    ) -> crate::error::Result<()> {
        let notifier_item_proxy = StatusNotifierItemProxy::builder(connection)
            .destination(destination)?
            .path(path)?
            .build()
            .await?;

        let dbus_proxy = DBusProxy::new(connection).await?;

        let mut disconnect_stream = dbus_proxy.receive_name_owner_changed().await?;

        let mut props_changed = notifier_item_proxy.receive_all_signals().await?;

        loop {
            tokio::select! {
                Some(change) = props_changed.next() => {
                    if let Some(event) = Self::get_update_event(change, &properties_proxy).await {
                        debug!("[{destination}{path}] received property change: {event:?}");
                        tx.send(Event::Update(destination.to_string(), event))?;
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

                            tx.send(Event::Remove(destination.to_string()))?;
                            break Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Gets the update event for a DBus properties change message.
    async fn get_update_event(
        change: Arc<Message>,
        properties_proxy: &PropertiesProxy<'_>,
    ) -> Option<UpdateEvent> {
        let member = change.member()?;

        let property = properties_proxy
            .get(
                InterfaceName::from_static_str(PROPERTIES_INTERFACE)
                    .expect("to be valid interface name"),
                member.as_str(),
            )
            .await
            .ok()?;

        debug!("received tray item update: {member} -> {property:?}");

        use UpdateEvent::*;
        match member.as_str() {
            "NewAttentionIcon" => Some(AttentionIcon(property.to_string())),
            "NewIcon" => Some(Icon(property.to_string())),
            "NewOverlayIcon" => Some(OverlayIcon(property.to_string())),
            "NewStatus" => Some(Status(
                property
                    .downcast_ref::<str>()
                    .map(item::Status::from)
                    .unwrap_or_default(),
            )),
            "NewTitle" => Some(Title(property.to_string())),
            // "NewTooltip" => Some(Tooltip(
            //     property
            //         .downcast_ref::<Structure>()
            //         .map(status_notifier_item::Tooltip::from),
            // )),
            _ => {
                warn!("received unhandled update event: {member}");
                None
            }
        }
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

        let menu = dbus_menu_proxy.get_layout(0, 10, &[]).await.unwrap();
        let menu = TrayMenu::try_from(menu)?;

        if let Some((_, menu_cache)) = items.lock().expect("to get lock").get_mut(&destination) {
            menu_cache.replace(menu.clone());
        } else {
            error!("could not find item in state");
        }

        tx.send(Event::Update(
            destination.to_string(),
            UpdateEvent::Menu(menu),
        ))?;

        let mut props_changed = dbus_menu_proxy.receive_all_signals().await?;

        while let Some(change) = props_changed.next().await {
            debug!("[{destination}{menu_path}] received menu change: {change:?}");

            match change.member() {
                Some(name) if name == "LayoutUpdated" => {
                    let menu = dbus_menu_proxy.get_layout(0, 10, &[]).await.unwrap();
                    let menu = TrayMenu::try_from(menu)?;

                    if let Some((_, menu_cache)) =
                        items.lock().expect("to get lock").get_mut(&destination)
                    {
                        menu_cache.replace(menu.clone());
                    } else {
                        error!("could not find item in state");
                    }

                    tx.send(Event::Update(
                        destination.to_string(),
                        UpdateEvent::Menu(menu),
                    ))?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Subscribes to the events broadcast channel,
    /// returning a new receiver.
    ///
    /// Once the client is dropped, the receiver will close.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Gets all current items, including their menus if present.
    pub fn items(&self) -> Arc<Mutex<State>> {
        self.items.clone()
    }

    /// Sends an activate request for a menu item.
    pub async fn activate(&self, req: ActivateRequest) -> crate::error::Result<()> {
        let dbus_menu_proxy = DBusMenuProxy::builder(&self.connection)
            .destination(req.address)
            .unwrap()
            .path(req.menu_path)?
            .build()
            .await?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time to flow forwards");

        dbus_menu_proxy
            .event(
                req.submenu_id,
                "clicked",
                &Value::I32(32),
                timestamp.as_secs() as u32,
            )
            .await?;

        Ok(())
    }
}
