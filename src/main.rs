use layer1::{client::NetClientLayer1, server::NetServerLayer1};
use std::time::Duration;

mod layer1;

async fn server() {
    let mut net_server = NetServerLayer1::new().await;

    let mut count = 0;
    loop {
        count += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let messages = net_server.dequeue().await;

        if !messages.is_empty() {
            println!(
                "{} messages: {:?}",
                messages.len(),
                messages
                    .iter()
                    .map(|message| (
                        message.0.clone(),
                        String::from_utf8(message.1.as_ref().unwrap().clone()).unwrap()
                    ))
                    .collect::<Vec<_>>()
            );
        }

        let name = {
            let write_txs = net_server.write_txs.lock().await;
            let keys = write_txs.keys().collect::<Vec<_>>();
            if keys.is_empty() {
                None
            } else {
                let index = count % keys.len();
                Some(keys[index].clone())
            }
        };
        if let Some(name) = name {
            println!("Sending message to {}", name.clone());
            net_server
                .enqueue(&name, "Hello from the server.".as_bytes().to_vec())
                .await;
        }
    }
}

async fn client() {
    let mut net_client = NetClientLayer1::new().await;

    loop {
        tokio::time::sleep(Duration::from_millis(800)).await;

        let messages = net_client.dequeue();

        if !messages.is_empty() {
            println!("{} messages: {:?}", messages.len(), messages);
            net_client.enqueue(
                format!("Got {} message(s).", messages.len())
                    .as_bytes()
                    .to_vec(),
            );
        }
    }
}

#[tokio::main]
async fn main() {
    if std::env::args()
        .find(|arg| arg.eq_ignore_ascii_case("server"))
        .is_some()
    {
        server().await;
    } else {
        client().await;
    }
}
