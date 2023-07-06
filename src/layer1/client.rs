use crate::layer1::utils::{read, write};
use tokio::{
    io::{self, BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};

pub struct NetClientLayer1 {
    error_rx: Receiver<io::Error>,
    read_rx: Receiver<Vec<u8>>,
    write_tx: Sender<Vec<u8>>,
}
impl NetClientLayer1 {
    pub async fn new() -> Result<Self, io::Error> {
        let stream = TcpStream::connect("127.0.0.1:5555").await?;

        let (error_tx, error_rx) = tokio::sync::mpsc::channel(128);
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(1024);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(1024);

        let (read_half, write_half) = stream.into_split();

        {
            let error_tx = error_tx.clone();
            tokio::spawn(async move {
                let mut writer = BufWriter::new(write_half);
                loop {
                    let bytes = write_rx.recv().await;
                    match write(&mut writer, bytes.unwrap()).await {
                        Ok(_) => {}
                        Err(error) => {
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
                            break;
                        }
                    }
                }

                println!("Shutting down writer");
            });
        }

        {
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
                            break;
                        }
                    }
                }

                println!("Shutting down reader");
            });
        }

        Ok(Self {
            error_rx,
            read_rx,
            write_tx,
        })
    }
    pub fn dequeue(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        while let Ok(message) = self.read_rx.try_recv() {
            messages.push(message);
        }
        messages
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
            Ok(_) => {}
            Err(_) => {}
        };
    }
}
