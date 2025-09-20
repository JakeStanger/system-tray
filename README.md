# System Tray

An async implementation of the `StatusNotifierItem` and `DbusMenu` protocols for building system trays.

Requires Tokio.

## Example

```rust
use system_tray::client::Client;

#[tokio::main]
async fn main() {
    let client = Client::new().await.unwrap();
    let mut tray_rx = client.subscribe();

    let initial_items = client.items();
    
    // do something with initial items...
    
    while let Ok(ev) = tray_rx.recv().await {
        println!("{ev:?}"); // do something with event...
    }
}
```

### `dbusmenu-gtk3`

Although the library provides a built-in Rust-native implementation of the `DBusMenu` protocol,
if you are creating a menu UI, you need to parse the whole tree set up each element, and track all changes manually.

To circumvent this, bindings to the `dbusmenu-gtk3` system library are included. 
When the feature of the same name is enabled, you can listen for `UpdateEvent::MenuConnect`
and create the GTK element based on that:

```rust
fn on_update(update: system_tray::Event) {
    match update {
        Event::Update(address, UpdateEvent::MenuConnect(menu)) => {
            let menu: gtk::auto::Menu = system_tray::gtk_menu::Menu::new(&address, &menu);
            // do something with the menu element
        }
    }
}
```

> [!NOTE]
> This feature is disabled by default to reduce compilation times.
> You must have `libdbusmenu-gtk3` installed on your system to build and use this feature.
> No equivalent library exists for GTK4.

## Attributions

Some of the code in this repository, namely the SNI host, is taken from [eww](https://github.com/elkowar/eww/blob/50ec181fc7ff2a68d6330e8897de2c5179575935/crates/notifier_host/src/host.rs) 
under [MIT](https://github.com/elkowar/eww/blob/master/LICENSE).
Many thanks to elkowar. 