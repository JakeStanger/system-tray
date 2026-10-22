use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use zbus::connection::Builder;
use zbus::message::Type;
use zbus::MessageStream;

// A private bus keeps these lifecycle tests independent of the desktop session.
struct TestBus {
    daemon: Child,
    address: String,
}

impl TestBus {
    fn new() -> Self {
        let mut daemon = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon must be installed to run lifecycle tests");
        let mut address = String::new();
        BufReader::new(daemon.stdout.take().expect("piped stdout"))
            .read_line(&mut address)
            .expect("read private bus address");
        Self {
            daemon,
            address: address.trim().to_owned(),
        }
    }

    async fn connect(&self) -> Connection {
        Builder::address(self.address.as_str())
            .expect("valid private bus address")
            .build()
            .await
            .expect("connect to private bus")
    }
}

impl Drop for TestBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

struct TestItem;

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl TestItem {
    #[zbus(property)]
    fn id(&self) -> &str {
        "disconnect-test"
    }

    #[zbus(property)]
    fn title(&self) -> &str {
        "Disconnect test"
    }
}

// Like KDE's watcher, this interface has no explicit unregister method.
struct ExternalWatcher;

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl ExternalWatcher {
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

async fn disconnect_item(bus: &TestBus, watcher: &Connection) {
    let connection = bus.connect().await;
    let item = bus.connect().await;
    item.object_server()
        .at(ITEM_OBJECT, TestItem)
        .await
        .unwrap();
    let destination = item.unique_name().unwrap().to_string();
    let items = TrayItemMap::new();
    let (tx, mut events) = broadcast::channel(32);
    timeout(
        Duration::from_secs(5),
        Client::handle_item(&destination, connection.clone(), tx, items.clone()),
    )
    .await
    .expect("initialize item")
    .unwrap();
    assert!(
        matches!(events.recv().await.unwrap(), Event::Add(address, _) if address == destination)
    );

    // Add is emitted before the property subscriptions are installed. Observe
    // an update before closing the item so its disconnect cannot race setup.
    timeout(Duration::from_secs(5), async {
        let mut tick = tokio::time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    item.emit_signal(
                        None::<&str>, ITEM_OBJECT, PROPERTIES_INTERFACE, "NewTitle", &(),
                    ).await.unwrap();
                }
                event = events.recv() => {
                    if matches!(event.unwrap(), Event::Update(_, UpdateEvent::Title(_))) {
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("client must begin watching the item");

    let mut watcher_messages = MessageStream::from(watcher);
    item.close().await.unwrap();
    timeout(Duration::from_secs(5), async {
        loop {
            if let Event::Remove(address) = events.recv().await.unwrap() {
                assert_eq!(address, destination);
                break;
            }
        }
    })
    .await
    .expect("client must remove the disconnected item");
    assert!(
        items.clear_items().is_empty(),
        "client must remove cached item data"
    );

    // A round trip after Remove bounds the messages to inspect without relying
    // on a sleep. The previous implementation sends UnregisterStatusNotifierItem
    // before publishing Remove, even when the watcher does not support it.
    connection
        .call_method(
            Some(names::WATCHER_BUS),
            names::WATCHER_OBJECT,
            Some("org.freedesktop.DBus.Peer"),
            "Ping",
            &(),
        )
        .await
        .unwrap();
    timeout(Duration::from_secs(5), async {
        while let Some(message) = watcher_messages.next().await {
            let message = message.unwrap();
            if message.message_type() == Type::MethodCall {
                let header = message.header();
                let member = header.member().unwrap().as_str();
                assert_ne!(member, "UnregisterStatusNotifierItem");
                if member == "Ping" {
                    return;
                }
            }
        }
        panic!("watcher message stream ended before the barrier");
    })
    .await
    .expect("watcher must receive the barrier");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_does_not_call_external_watcher() {
    let bus = TestBus::new();
    let watcher = bus.connect().await;
    watcher
        .object_server()
        .at(names::WATCHER_OBJECT, ExternalWatcher)
        .await
        .unwrap();
    watcher.request_name(names::WATCHER_BUS).await.unwrap();

    disconnect_item(&bus, &watcher).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_does_not_call_builtin_watcher() {
    let bus = TestBus::new();
    let watcher = bus.connect().await;
    StatusNotifierWatcher::new()
        .attach_to(&watcher)
        .await
        .unwrap();

    disconnect_item(&bus, &watcher).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_watcher_unregisters_items_on_service_exit() {
    let bus = TestBus::new();
    let watcher = bus.connect().await;
    StatusNotifierWatcher::new()
        .attach_to(&watcher)
        .await
        .unwrap();
    let host = bus.connect().await;
    let proxy = StatusNotifierWatcherProxy::builder(&host)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .unwrap();
    let mut removed = proxy
        .receive_status_notifier_item_unregistered()
        .await
        .unwrap();
    let item = bus.connect().await;
    let item_proxy = StatusNotifierWatcherProxy::new(&item).await.unwrap();
    item_proxy
        .register_status_notifier_item(ITEM_OBJECT)
        .await
        .unwrap();
    let address = format!("{}{ITEM_OBJECT}", item.unique_name().unwrap());
    assert_eq!(
        proxy
            .registered_status_notifier_items()
            .await
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&address)
    );

    item.close().await.unwrap();
    let signal = timeout(Duration::from_secs(5), removed.next())
        .await
        .expect("watcher must announce the item removal without a client request")
        .unwrap();
    assert_eq!(signal.args().unwrap().service, address);
    assert!(proxy
        .registered_status_notifier_items()
        .await
        .unwrap()
        .is_empty());
}
