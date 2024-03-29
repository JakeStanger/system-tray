use std::collections::HashSet;
use tracing::{debug, error};
use zbus::{dbus_interface, MessageHeader, SignalContext};

/// StatusNotifierWatcher server implementation.
///
/// This is created as a connection on the session DBus.
/// To use, a proxy must be obtained and used as a client.
///
/// You will want to call `register_status_notifier_host` on the proxy
/// to notify the server you wish to receive tray items.
pub(crate) struct StatusNotifierWatcher {
    host_registered: bool,
    registered_hosts: HashSet<String>,
    registered_items: HashSet<String>,
    protocol_version: i32,
}

impl StatusNotifierWatcher {
    pub(crate) fn new() -> Self {
        Self {
            host_registered: false,
            protocol_version: 0,
            registered_items: HashSet::new(),
            registered_hosts: HashSet::new(),
        }
    }
}

#[dbus_interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    /// Registers a new `StatusNotifierHost` on the watcher.
    ///
    /// This is tracked by the server,
    /// which then sends a `StatusNotifierHostRegistered` signal.
    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_context)] context: SignalContext<'_>,
    ) {
        debug!("host registered: {service}");
        self.host_registered = true;
        self.registered_hosts.insert(service.to_string());

        if let Err(err) = self
            .is_status_notifier_host_registered_changed(&context)
            .await
        {
            error!("{err:?}");
        }

        if let Err(err) = Self::status_notifier_item_registered(&context, service).await {
            error!("{err:?}");
        }
    }

    /// Registers a new `StatusNotifierItem` on the watcher.
    ///
    /// This is tracked by the server,
    /// which then sends a `StatusNotifierItemRegistered` signal.
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: MessageHeader<'_>,
        #[zbus(signal_context)] context: SignalContext<'_>,
    ) {
        let address = header
            .sender()
            .expect("Failed to get message sender in header")
            .map(|name| name.to_string())
            .expect("Failed to get unique name for notifier");

        debug!("registered item: {service} | {address}");

        let notifier_item = if address == service {
            address
        } else {
            format!("{}{}", address, service)
        };

        if let Err(err) = Self::status_notifier_item_registered(&context, &notifier_item).await {
            error!("{err:?}");
        }

        self.registered_items.insert(notifier_item);
    }

    async fn unregister_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_context)] context: SignalContext<'_>,
    ) {
        debug!("received host unregister: {service}");

        self.registered_hosts.remove(service);

        if let Err(err) = Self::status_notifier_host_unregistered(&context).await {
            error!("{err:?}")
        }
    }

    async fn unregister_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(signal_context)] context: SignalContext<'_>,
    ) {
        debug!("received item unregister: {service}");

        self.registered_items.remove(service);

        if let Err(err) = Self::status_notifier_item_unregistered(&context, service).await {
            error!("{err:?}");
        }
    }

    #[dbus_interface(signal)]
    async fn status_notifier_host_registered(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_host_unregistered(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_item_registered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_item_unregistered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[dbus_interface(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        self.host_registered
    }

    #[dbus_interface(property)]
    async fn protocol_version(&self) -> i32 {
        self.protocol_version
    }

    #[dbus_interface(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registered_items.iter().cloned().collect()
    }
}
