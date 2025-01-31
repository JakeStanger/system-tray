use crate::names;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use zbus::message::Header;
use zbus::object_server::Interface;
use zbus::object_server::SignalEmitter;
use zbus::{export::ordered_stream::OrderedStreamExt, interface, Connection};

/// An instance of [`org.kde.StatusNotifierWatcher`]. It only tracks what tray items and trays
/// exist, and doesn't have any logic for displaying items (for that, see [`Host`][`crate::Host`]).
///
/// While this is usually run alongside the tray, it can also be used standalone.
///
/// [`org.kde.StatusNotifierWatcher`]: https://freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/
#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    tasks: tokio::task::JoinSet<()>,

    hosts: Arc<Mutex<HashSet<String>>>,
    items: Arc<Mutex<HashSet<String>>>,
}

/// Implementation of the `StatusNotifierWatcher` service.
///
/// Methods and properties correspond to methods and properties on the DBus service that can be
/// used by others, while signals are events that we generate that other services listen to.
#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    /// RegisterStatusNotifierHost method
    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(connection)] con: &Connection,
        #[zbus(signal_emitter)] signal_emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        // TODO: right now, we convert everything to the unique bus name (something like :1.234).
        //  However, it might make more sense to listen to the actual name they give us, so that if
        //  the connection dissociates itself from the org.kde.StatusNotifierHost-{pid}-{nr} name
        //  but still remains around, we drop them as a host.
        //  (This also applies to RegisterStatusNotifierItem)

        let (service, _) = parse_service(service, hdr, con).await?;
        info!("new host: {}", service);

        let added_first = {
            // scoped around locking of hosts
            let mut hosts = self.hosts.lock().expect("mutex lock should succeed");
            if !hosts.insert(service.to_string()) {
                // we're already tracking them
                return Ok(());
            }
            hosts.len() == 1
        };

        if added_first {
            self.is_status_notifier_host_registered_changed(&signal_emitter)
                .await?;
        }
        StatusNotifierWatcher::status_notifier_host_registered(&signal_emitter).await?;

        self.tasks.spawn({
            let hosts = self.hosts.clone();
            let signal_emitter = signal_emitter.to_owned();
            let con = con.to_owned();
            async move {
                if let Err(e) = wait_for_service_exit(&con, service.as_ref().into()).await {
                    error!("failed to wait for service exit: {}", e);
                }
                info!("lost host: {}", service);

                let removed_last = {
                    let mut hosts = hosts.lock().expect("mutex lock should succeed");
                    let did_remove = hosts.remove(service.as_str());
                    did_remove && hosts.is_empty()
                };

                if removed_last {
                    if let Err(e) =
                        StatusNotifierWatcher::is_status_notifier_host_registered_refresh(
                            &signal_emitter,
                        )
                        .await
                    {
                        error!("failed to signal Watcher: {}", e);
                    }
                }
                if let Err(e) =
                    StatusNotifierWatcher::status_notifier_host_unregistered(&signal_emitter).await
                {
                    error!("failed to signal Watcher: {}", e);
                }
            }
        });

        Ok(())
    }

    /// StatusNotifierHostRegistered signal.
    #[zbus(signal)]
    async fn status_notifier_host_registered(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    /// StatusNotifierHostUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_host_unregistered(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    /// IsStatusNotifierHostRegistered property
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        let hosts = self.hosts.lock().expect("mutex lock should succeed");
        !hosts.is_empty()
    }

    /// RegisterStatusNotifierItem method
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(connection)] con: &Connection,
        #[zbus(signal_emitter)] signal_emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let (service, objpath) = parse_service(service, hdr, con).await?;
        let service = zbus::names::BusName::Unique(service);

        let item = format!("{service}{objpath}");

        {
            let mut items = self.items.lock().expect("mutex lock should succeed");
            if !items.insert(item.clone()) {
                // we're already tracking them
                info!("new item: {} (duplicate)", item);
                return Ok(());
            }
        }
        info!("new item: {}", item);

        self.registered_status_notifier_items_changed(&signal_emitter)
            .await?;
        StatusNotifierWatcher::status_notifier_item_registered(&signal_emitter, item.as_ref())
            .await?;

        self.tasks.spawn({
            let items = self.items.clone();
            let signal_emitter = signal_emitter.to_owned();
            let con = con.to_owned();
            async move {
                if let Err(e) = wait_for_service_exit(&con, service.as_ref()).await {
                    error!("failed to wait for service exit: {}", e);
                }
                debug!("gone item: {}", &item);

                {
                    let mut items = items.lock().expect("mutex lock should succeed");
                    items.remove(&item);
                }

                if let Err(e) =
                    StatusNotifierWatcher::registered_status_notifier_items_refresh(&signal_emitter)
                        .await
                {
                    error!("failed to signal Watcher: {}", e);
                }
                if let Err(e) = StatusNotifierWatcher::status_notifier_item_unregistered(
                    &signal_emitter,
                    item.as_ref(),
                )
                .await
                {
                    error!("failed to signal Watcher: {}", e);
                }
            }
        });

        Ok(())
    }

    async fn unregister_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(connection)] con: &Connection,
        #[zbus(signal_emitter)] context: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        debug!("received item unregister: {service}");

        let (service, objpath) = parse_service(service, hdr, con).await?;
        let service = zbus::names::BusName::Unique(service);

        let item = format!("{service}{objpath}");

        self.items
            .lock()
            .expect("mutex lock should succeed")
            .remove(&item);

        if let Err(err) = Self::status_notifier_item_unregistered(&context, &item).await {
            error!("{err:?}");
        }

        Ok(())
    }

    /// StatusNotifierItemRegistered signal
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// RegisteredStatusNotifierItems property
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.items.lock().expect("mutex lock should succeed");
        items.iter().cloned().collect()
    }

    /// ProtocolVersion property
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

impl StatusNotifierWatcher {
    /// Create a new Watcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach and run the Watcher (in the background) on a connection.
    pub async fn attach_to(self, con: &zbus::Connection) -> zbus::Result<()> {
        if !con.object_server().at(names::WATCHER_OBJECT, self).await? {
            return Err(zbus::Error::Failure(format!(
                "Object already exists at {} on this connection -- is StatusNotifierWatcher already running?",
                names::WATCHER_OBJECT
            )));
        }

        // not AllowReplacement, not ReplaceExisting, not DoNotQueue
        let flags: [zbus::fdo::RequestNameFlags; 0] = [];
        match con
            .request_name_with_flags(names::WATCHER_BUS, flags.into_iter().collect())
            .await
        {
            Ok(_) | Err(zbus::Error::NameTaken) => Ok(()), // defer to existing
            Err(e) => Err(e),
        }
    }

    /// Equivalent to `is_status_notifier_host_registered_invalidate`, but without requiring
    /// `self`.
    async fn is_status_notifier_host_registered_refresh(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()> {
        zbus::fdo::Properties::properties_changed(
            signal_emitter,
            Self::name(),
            std::collections::HashMap::new(),
            Cow::Borrowed(&["IsStatusNotifierHostRegistered"]),
        )
        .await
    }

    /// Equivalent to `registered_status_notifier_items_invalidate`, but without requiring `self`.
    async fn registered_status_notifier_items_refresh(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()> {
        zbus::fdo::Properties::properties_changed(
            signal_emitter,
            Self::name(),
            std::collections::HashMap::new(),
            Cow::Borrowed(&["RegisteredStatusNotifierItems"]),
        )
        .await
    }
}

/// Decode the service name that others give to us, into the [bus
/// name](https://dbus2.github.io/zbus/concepts.html#bus-name--service-name) and the [object
/// path](https://dbus2.github.io/zbus/concepts.html#objects-and-object-paths) within the
/// connection.
///
/// The freedesktop.org specification has the format of this be just the bus name, however some
/// status items pass non-conforming values. One common one is just the object path.
async fn parse_service<'a>(
    service: &'a str,
    hdr: Header<'_>,
    con: &Connection,
) -> zbus::fdo::Result<(zbus::names::UniqueName<'static>, &'a str)> {
    if service.starts_with('/') {
        // they sent us just the object path
        if let Some(sender) = hdr.sender() {
            Ok((sender.to_owned(), service))
        } else {
            warn!("unknown sender");
            Err(zbus::fdo::Error::InvalidArgs("Unknown bus address".into()))
        }
    } else {
        // parse the bus name they gave us
        let busname: zbus::names::BusName = match service.try_into() {
            Ok(x) => x,
            Err(e) => {
                warn!("received invalid bus name {:?}: {}", service, e);
                return Err(zbus::fdo::Error::InvalidArgs(e.to_string()));
            }
        };

        if let zbus::names::BusName::Unique(unique) = busname {
            Ok((unique.to_owned(), names::ITEM_OBJECT))
        } else {
            // they gave us a "well-known name" like org.kde.StatusNotifierHost-81830-0, we need to
            // convert this into the actual identifier for their bus (e.g. :1.234), so that even if
            // they remove that well-known name it's fine.
            let dbus = zbus::fdo::DBusProxy::new(con).await?;
            match dbus.get_name_owner(busname).await {
                Ok(owner) => Ok((owner.into_inner(), names::ITEM_OBJECT)),
                Err(e) => {
                    warn!("failed to get owner of {:?}: {}", service, e);
                    Err(e)
                }
            }
        }
    }
}

/// Wait for a `DBus` service to disappear
async fn wait_for_service_exit(
    con: &Connection,
    service: zbus::names::BusName<'_>,
) -> zbus::fdo::Result<()> {
    let dbus = zbus::fdo::DBusProxy::new(con).await?;
    let mut owner_changes = dbus
        .receive_name_owner_changed_with_args(&[(0, &service)])
        .await?;

    if !dbus.name_has_owner(service.as_ref()).await? {
        // service has already disappeared
        return Ok(());
    }

    while let Some(sig) = owner_changes.next().await {
        let args = sig.args()?;
        if args.new_owner().is_none() {
            break;
        }
    }

    Ok(())
}
