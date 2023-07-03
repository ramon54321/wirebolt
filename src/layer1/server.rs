use crate::layer1::utils::{read, write};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{self, BufReader, BufWriter},
    net::TcpListener,
    select,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

pub struct NetServerLayer1 {
    read_rxs: Arc<Mutex<HashMap<String, Receiver<Result<Vec<u8>, io::Error>>>>>,
    pub write_txs: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
}
impl NetServerLayer1 {
    pub async fn new() -> Self {
        let tcp_listener = TcpListener::bind("127.0.0.1:5555")
            .await
            .expect("Unable to bind listener to socket.");

        let read_rxs = Arc::new(Mutex::new(HashMap::new()));
        let write_txs = Arc::new(Mutex::new(HashMap::new()));

        // -- Connection Listener
        {
            let read_rxs = read_rxs.clone();
            let write_txs = write_txs.clone();
            tokio::spawn(async move {
                loop {
                    let (mut stream, address) = tcp_listener
                        .accept()
                        .await
                        .expect("Unable to bind to connecting socket.");
                    let name = address.to_string();

                    let (read_tx, read_rx) = tokio::sync::mpsc::channel(64);
                    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(64);

                    read_rxs.lock().await.insert(name.clone(), read_rx);
                    write_txs.lock().await.insert(name.clone(), write_tx);

                    {
                        let read_rxs = read_rxs.clone();
                        let write_txs = write_txs.clone();
                        tokio::spawn(async move {
                            let (read_half, write_half) = stream.split();

                            let mut writer = BufWriter::new(write_half);
                            let mut reader = BufReader::new(read_half);

                            loop {
                                select! {
                                    bytes = write_rx.recv() => {
                                        match write(&mut writer, bytes.unwrap()).await {
                                            Ok(_) => {},
                                            Err(error) => eprintln!("Error in writing to socket: {}", error),
                                        }
                                    }
                                    response = read(&mut reader) => {
                                        match response {
                                            Ok(bytes) => {
                                                match read_tx.try_send(Ok(bytes)) {
                                                    Ok(_) => {},
                                                    Err(error) => {
                                                        eprintln!("Error in channeling read: {}", error);
                                                        panic!();
                                                    }
                                                }
                                            },
                                            Err(error) => {
                                                // TODO: This is pointless because the channel gets
                                                // removed. Perhaps delay channel removal somehow
                                                match read_tx.try_send(Err(error)) {
                                                    Ok(_) => {},
                                                    Err(error) => {
                                                        eprintln!("Error in channeling read error: {}", error);
                                                        panic!();
                                                    }
                                                }
                                                read_rxs.lock().await.remove(&name.clone());
                                                write_txs.lock().await.remove(&name.clone());
                                                break;
                                            },
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            });
        }

        Self {
            read_rxs,
            write_txs,
        }
    }
    pub async fn dequeue(&mut self) -> Vec<(String, Result<Vec<u8>, io::Error>)> {
        let mut messages = Vec::new();
        for (name, read_rx) in self.read_rxs.lock().await.iter_mut() {
            while let Ok(message) = read_rx.try_recv() {
                messages.push((name.clone(), message));
            }
        }
        messages
    }
    pub async fn enqueue(&mut self, name: &str, bytes: Vec<u8>) {
        self.write_txs
            .lock()
            .await
            .get_mut(name)
            .unwrap()
            .try_send(bytes)
            .unwrap();
    }
}
