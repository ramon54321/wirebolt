use layer1::{client::NetClientLayer1, server::NetServerLayer1};

mod layer1;

async fn server() {
    let Ok(mut net_server) = NetServerLayer1::new().await else { return; };

    let mut count = 0;
    loop {
        let messages = net_server.dequeue().await;

        for message in messages.iter() {
            // println!("MAIN: {:?}", String::from_utf8(message.to_owned()).unwrap());
        }

        let keys = net_server
            .write_txs
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        if keys.is_empty() {
            continue;
        }

        count += 1;
        for name in keys.iter() {
            let message = format!("Test-{}", count);
            net_server
                .enqueue(&name, message.clone().as_bytes().to_vec())
                .await;
        }

        if count % 1000 == 0 {
            println!("Sent {}", count);
        }
    }
}

async fn client() {
    let Ok(mut net_client) = NetClientLayer1::new().await else { return; };

    let mut message_count = 0;
    loop {
        let messages = net_client.dequeue();
        let errors = net_client.dequeue_errors();

        for error in errors.iter() {
            eprintln!("Handling error: {}", error);
        }

        for message in messages.iter() {
            message_count += 1;
            let text = String::from_utf8(message.to_owned()).unwrap();
            net_client
                .enqueue(format!("Response: {}", text).as_bytes().to_vec())
                .await;

            if message_count % 1000 == 0 {
                println!("Message: {}", String::from_utf8(message.clone()).unwrap());
            }
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
