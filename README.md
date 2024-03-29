# System Tray

An async implementation of the StatusNotifierItem and DbusMenu protocols for building system trays.

Requires Tokio.

## Example

```rust
#[tokio::main]
async fn main() {
    let client = context.client::<tray::Client>();
    let mut tray_rx = client.subscribe();

    let initial_items = client.items();
    
    // do something with initial items
    
    while let Ok(ev) = tray_rx.recv().await {
        println!("{ev:?}"); // do something with event
    }
}
```