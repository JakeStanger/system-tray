use system_tray::client::Client;

#[tokio::main]
async fn main() {
    let client = Client::new().await.unwrap();
    let mut tray_rx = client.subscribe();

    let initial_items = client.items();

    // do something with initial items...
    drop(initial_items);

    while let Ok(ev) = tray_rx.recv().await {
        println!("{ev:?}"); // do something with event...
    }
}
