use std::time::Duration;
use tokhost::{NetClient, NetServer};

async fn server() {
    let Ok(mut net_server) = NetServer::new("127.0.0.1:5555").await else { return; };

    let mut count = 0;
    loop {
        // -- Do not do anything if there are no connections
        if net_server.connection_count().await == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            continue;
        }

        // -- Read errors
        let errors = net_server.dequeue_errors().await;
        for error in errors.iter() {
            println!("Error Handled: {:?}", error);
        }

        // -- Read messages
        let messages = net_server.dequeue().await;
        for _message in messages.iter() {}

        // -- Send messages
        let message = format!("Message-{}", count);
        net_server
            .broadcast(message.clone().as_bytes().to_vec())
            .await;

        // -- Report every thousand messages sent
        if count % 1000 == 0 {
            println!("Sent {} messages.", count);
        }

        count += 1;
    }
}

async fn client() {
    let Ok(mut net_client) = NetClient::new("127.0.0.1:5555").await else { return; };

    let mut message_count = 0;
    loop {
        if !net_client.is_connected() {
            break;
        }

        // -- Read errors
        let errors = net_client.dequeue_errors();
        for error in errors.iter() {
            println!("Error Handled: {:?}", error);
        }

        // -- Read messages
        let messages = net_client.dequeue();
        for message in messages.iter() {
            message_count += 1;
            // -- Convert message to string
            let text = String::from_utf8(message.to_owned()).unwrap();

            // -- Send message back to server
            net_client
                .enqueue(format!("Response: {}", text).as_bytes().to_vec())
                .await;

            // -- Report every thousand messages
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
