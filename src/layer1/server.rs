use crate::layer1::utils::{read, write};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{self, BufReader, BufWriter},
    join,
    net::TcpListener,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

#[derive(Debug)]
pub enum NetServerInfo {
    Connection,
    Disconnection,
}
pub struct NetServerLayer1 {
    connections: Arc<Mutex<HashSet<String>>>,
    info_rx: Receiver<(String, NetServerInfo)>,
    error_rx: Receiver<(String, io::Error)>,
    read_rx: Receiver<(String, Vec<u8>)>,
    write_txs: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
}
impl NetServerLayer1 {
    pub async fn new(address: &str) -> Result<Self, io::Error> {
        let tcp_listener = TcpListener::bind(address).await?;

        let (info_tx, info_rx) = tokio::sync::mpsc::channel(256);
        let (error_tx, error_rx) = tokio::sync::mpsc::channel(256);
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(1024);
        let write_txs = Arc::new(Mutex::new(HashMap::new()));
        let connections = Arc::new(Mutex::new(HashSet::new()));

        // -- Connection Listener
        {
            let write_txs = write_txs.clone();
            let connections = connections.clone();
            tokio::spawn(async move {
                loop {
                    let (stream, address) = tcp_listener
                        .accept()
                        .await
                        .expect("Unable to bind to connecting socket.");
                    let name = address.to_string();

                    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(1024);

                    write_txs.lock().await.insert(name.clone(), write_tx);
                    connections.lock().await.insert(name.clone());

                    {
                        let (read_half, write_half) = stream.into_split();

                        let writer_task = {
                            let name = name.clone();
                            let error_tx = error_tx.clone();
                            let writer_task = tokio::spawn(async move {
                                let mut writer = BufWriter::new(write_half);
                                loop {
                                    let bytes = write_rx.recv().await;
                                    match write(&mut writer, bytes.unwrap()).await {
                                        Ok(_) => {}
                                        Err(error) => {
                                            error_tx.send((name.clone(), error)).await
                                                .expect("Could not send error. Is the error_tx still available?");
                                            break;
                                        }
                                    }
                                }
                                // println!("Shutting down writer.");
                            });
                            writer_task
                        };

                        let reader_task = {
                            let name = name.clone();
                            let error_tx = error_tx.clone();
                            let read_tx = read_tx.clone();
                            let reader_task = tokio::spawn(async move {
                                let mut reader = BufReader::new(read_half);
                                loop {
                                    let response = read(&mut reader).await;
                                    match response {
                                        Ok(bytes) => read_tx
                                            .send((name.clone(), bytes))
                                            .await
                                            .expect(
                                            "Could not send read. Is the read_tx still available?",
                                        ),
                                        Err(error) => {
                                            error_tx.send((name.clone(), error)).await
                                                .expect("Could not send error. Is the error_tx still available?");
                                            break;
                                        }
                                    }
                                }
                                // println!("Shutting down reader.");
                            });
                            reader_task
                        };

                        // -- Wait for both Reader and Writer to finish, then clean up the server
                        {
                            let name = name.clone();
                            let connections = connections.clone();
                            let info_tx = info_tx.clone();
                            let write_txs = write_txs.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                loop {
                                    if writer_task.is_finished() {
                                        reader_task.abort();
                                        // println!("Aborting reader.");
                                        break;
                                    }
                                    if reader_task.is_finished() {
                                        writer_task.abort();
                                        // println!("Aborting writer.");
                                        break;
                                    }
                                }
                                let (_, _) = join!(writer_task, reader_task);
                                // println!("Tasks joined. Cleaning up server.");
                                write_txs.lock().await.remove(&name.clone());
                                connections.lock().await.remove(&name.clone());
                                info_tx
                                    .send((name.clone(), NetServerInfo::Disconnection))
                                    .await
                                    .expect("Could not send info. Is the info_tx still available?");
                            });
                        }

                        info_tx
                            .send((name.clone(), NetServerInfo::Connection))
                            .await
                            .expect("Could not send info. Is the info_tx still available?");
                    }
                }
            });
        }

        Ok(Self {
            connections,
            info_rx,
            error_rx,
            read_rx,
            write_txs,
        })
    }
    pub async fn connection_count(&self) -> usize {
        self.connections.lock().await.len()
    }
    pub fn dequeue(&mut self) -> Vec<(String, Vec<u8>)> {
        let mut messages = Vec::new();
        while let Ok(message) = self.read_rx.try_recv() {
            messages.push(message);
        }
        messages
    }
    pub fn dequeue_info(&mut self) -> Vec<(String, NetServerInfo)> {
        let mut infos = Vec::new();
        while let Ok(info) = self.info_rx.try_recv() {
            infos.push(info);
        }
        infos
    }
    pub fn dequeue_errors(&mut self) -> Vec<(String, io::Error)> {
        let mut errors = Vec::new();
        while let Ok(error) = self.error_rx.try_recv() {
            errors.push(error);
        }
        errors
    }
    pub async fn enqueue(&mut self, connection: &str, bytes: Vec<u8>) {
        let txs = self.write_txs.lock().await;
        let tx = match txs.get(connection) {
            Some(tx) => tx,
            None => {
                // TODO: Perhaps an error should be propagated up if unable to get connection
                // Connection likely just disconnected after the enqueue was requested
                return;
            }
        };
        match tx.send(bytes).await {
            // -- No need to catch error. Client may have disconnected before tx completes
            // TODO: Perhaps the error should indeed be propagated up to the caller... maybe they
            // need the unsent bytes
            Ok(_) => {}
            Err(_) => {}
        };
    }
    pub async fn broadcast(&mut self, bytes: Vec<u8>) {
        let txs = self.write_txs.lock().await;
        for connection in self.connections.lock().await.iter() {
            let tx = match txs.get(connection) {
                Some(tx) => tx,
                None => {
                    // TODO: Perhaps an error should be propagated up if unable to get connection
                    // Connection likely just disconnected after the broadcast was requested
                    continue;
                }
            };
            match tx.send(bytes.clone()).await {
                // -- No need to catch error. Client may have disconnected before tx completes
                // TODO: Perhaps the error should indeed be propagated up to the caller... maybe they
                // need the unsent bytes
                Ok(_) => {}
                Err(_) => {}
            };
        }
    }
}
