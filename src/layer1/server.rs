use crate::layer1::utils::{read, write};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{self, BufReader, BufWriter},
    net::TcpListener,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

pub struct NetServerLayer1 {
    error_rxs: Arc<Mutex<HashMap<String, Receiver<io::Error>>>>,
    read_rxs: Arc<Mutex<HashMap<String, Receiver<Vec<u8>>>>>,
    pub write_txs: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
}
impl NetServerLayer1 {
    pub async fn new() -> Result<Self, io::Error> {
        let tcp_listener = TcpListener::bind("127.0.0.1:5555").await?;

        let error_rxs = Arc::new(Mutex::new(HashMap::new()));
        let read_rxs = Arc::new(Mutex::new(HashMap::new()));
        let write_txs = Arc::new(Mutex::new(HashMap::new()));

        // -- Connection Listener
        {
            let error_rxs = error_rxs.clone();
            let read_rxs = read_rxs.clone();
            let write_txs = write_txs.clone();
            tokio::spawn(async move {
                loop {
                    let (stream, address) = tcp_listener
                        .accept()
                        .await
                        .expect("Unable to bind to connecting socket.");
                    let name = address.to_string();

                    let (error_tx, error_rx) = tokio::sync::mpsc::channel(64);
                    let (read_tx, read_rx) = tokio::sync::mpsc::channel(1024);
                    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(1024);

                    error_rxs.lock().await.insert(name.clone(), error_rx);
                    read_rxs.lock().await.insert(name.clone(), read_rx);
                    write_txs.lock().await.insert(name.clone(), write_tx);

                    {
                        let (read_half, write_half) = stream.into_split();

                        {
                            let name = name.clone();
                            let error_rxs = error_rxs.clone();
                            let read_rxs = read_rxs.clone();
                            let write_txs = write_txs.clone();
                            let error_tx = error_tx.clone();
                            tokio::spawn(async move {
                                let mut writer = BufWriter::new(write_half);
                                loop {
                                    let bytes = write_rx.recv().await;
                                    match write(&mut writer, bytes.unwrap()).await {
                                        Ok(_) => {}
                                        Err(error) => {
                                            // TODO: This is pointless because the channel gets
                                            // removed. Perhaps delay channel removal somehow
                                            match error_tx.send(error).await {
                                                // TODO: Should catch error - if this errors then
                                                // the error_rx is gone too early and the caller
                                                // will never get the error in the dequeue_errors
                                                // call
                                                Ok(_) => {}
                                                Err(_) => {}
                                            }
                                            error_rxs.lock().await.remove(&name.clone());
                                            read_rxs.lock().await.remove(&name.clone());
                                            write_txs.lock().await.remove(&name.clone());
                                            break;
                                        }
                                    }
                                }
                                println!("Shutting down writer");
                            });
                        }

                        {
                            let name = name.clone();
                            let error_rxs = error_rxs.clone();
                            let read_rxs = read_rxs.clone();
                            let write_txs = write_txs.clone();
                            let error_tx = error_tx.clone();
                            tokio::spawn(async move {
                                let mut reader = BufReader::new(read_half);
                                loop {
                                    let response = read(&mut reader).await;
                                    match response {
                                        Ok(bytes) => match read_tx.send(bytes).await {
                                            Ok(_) => {}
                                            Err(error) => {
                                                eprintln!("Error in channeling read: {}", error);
                                                panic!();
                                            }
                                        },
                                        Err(error) => {
                                            // TODO: This is pointless because the channel gets
                                            // removed. Perhaps delay channel removal somehow
                                            match error_tx.send(error).await {
                                                // TODO: Should catch error - if this errors then
                                                // the error_rx is gone too early and the caller
                                                // will never get the error in the dequeue_errors
                                                // call
                                                Ok(_) => {}
                                                Err(_) => {
                                                    panic!()
                                                }
                                            }
                                            error_rxs.lock().await.remove(&name.clone());
                                            read_rxs.lock().await.remove(&name.clone());
                                            write_txs.lock().await.remove(&name.clone());
                                            break;
                                        }
                                    }
                                }

                                println!("Shutting down reader");
                            });
                        }
                    }
                }
            });
        }

        Ok(Self {
            error_rxs,
            read_rxs,
            write_txs,
        })
    }
    pub async fn dequeue(&mut self) -> Vec<(String, Vec<u8>)> {
        let mut messages = Vec::new();
        for (name, read_rx) in self.read_rxs.lock().await.iter_mut() {
            while let Ok(message) = read_rx.try_recv() {
                messages.push((name.clone(), message));
            }
        }
        messages
    }
    pub async fn dequeue_errors(&mut self) -> Vec<(String, io::Error)> {
        let mut errors = Vec::new();
        for (name, error_rx) in self.error_rxs.lock().await.iter_mut() {
            while let Ok(error) = error_rx.try_recv() {
                errors.push((name.clone(), error));
            }
        }
        errors
    }
    pub async fn enqueue(&mut self, name: &str, bytes: Vec<u8>) {
        let mut txs = self.write_txs.lock().await;
        let tx = txs.get_mut(name).unwrap();
        match tx.send(bytes).await {
            // -- No need to catch error. Client may have disconnected before tx completes
            Ok(_) => {}
            Err(_) => {}
        };
    }
}
