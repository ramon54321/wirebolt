use crate::layer1::utils::{read, write};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{self, BufReader, BufWriter},
    join,
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};

#[derive(Debug)]
pub enum NetClientInfo {
    Connection,
    Disconnection,
}
pub struct NetClientLayer1 {
    is_connected: Arc<AtomicBool>,
    info_rx: Receiver<NetClientInfo>,
    error_rx: Receiver<io::Error>,
    read_rx: Receiver<Vec<u8>>,
    write_tx: Sender<Vec<u8>>,
}
impl NetClientLayer1 {
    pub async fn new(address: &str) -> Result<Self, io::Error> {
        let stream = TcpStream::connect(address).await?;

        let is_connected = Arc::new(AtomicBool::new(true));
        let (info_tx, info_rx) = tokio::sync::mpsc::channel(128);
        let (error_tx, error_rx) = tokio::sync::mpsc::channel(128);
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(1024);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(1024);

        let (read_half, write_half) = stream.into_split();

        let writer_task = {
            let error_tx = error_tx.clone();
            let writer_task = tokio::spawn(async move {
                let mut writer = BufWriter::new(write_half);
                loop {
                    let bytes = write_rx.recv().await;
                    match write(&mut writer, bytes.unwrap()).await {
                        Ok(_) => {}
                        Err(error) => {
                            error_tx
                                .send(error)
                                .await
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
            let error_tx = error_tx.clone();
            let reader_task = tokio::spawn(async move {
                let mut reader = BufReader::new(read_half);
                loop {
                    let response = read(&mut reader).await;
                    match response {
                        Ok(bytes) => read_tx
                            .send(bytes)
                            .await
                            .expect("Could not send read. Is the read_tx still available?"),
                        Err(error) => {
                            error_tx
                                .send(error)
                                .await
                                .expect("Could not send error. Is the error_tx still available?");
                            break;
                        }
                    }
                }
                // println!("Shutting down reader.");
            });
            reader_task
        };

        // -- Wait for both Reader and Writer to finish
        {
            let is_connected = is_connected.clone();
            let info_tx = info_tx.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(50)).await;
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
                // println!("Tasks joined. Cleaning up client.");
                info_tx
                    .send(NetClientInfo::Disconnection)
                    .await
                    .expect("Could not send info. Is the info_tx still available?");
                is_connected.store(false, Ordering::Relaxed);
            });
        }

        info_tx
            .send(NetClientInfo::Connection)
            .await
            .expect("Could not send info. Is the info_tx still available?");

        Ok(Self {
            is_connected,
            info_rx,
            error_rx,
            read_rx,
            write_tx,
        })
    }
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }
    pub fn dequeue(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        while let Ok(message) = self.read_rx.try_recv() {
            messages.push(message);
        }
        messages
    }
    pub fn dequeue_info(&mut self) -> Vec<NetClientInfo> {
        let mut infos = Vec::new();
        while let Ok(info) = self.info_rx.try_recv() {
            infos.push(info);
        }
        infos
    }
    pub fn dequeue_errors(&mut self) -> Vec<io::Error> {
        let mut errors = Vec::new();
        while let Ok(error) = self.error_rx.try_recv() {
            errors.push(error);
        }
        errors
    }
    pub async fn enqueue(&mut self, bytes: Vec<u8>) {
        match self.write_tx.send(bytes).await {
            // -- No need to catch error. Client may have disconnected before tx completes
            // TODO: Perhaps the error should indeed be propagated up to the caller... maybe they
            // need the unsent bytes
            Ok(_) => {}
            Err(_) => {}
        };
    }
}
