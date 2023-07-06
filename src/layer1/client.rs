use crate::layer1::utils::{read, write};
use tokio::{
    io::{self, BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc::{error::SendError, Receiver, Sender},
};

pub struct NetClientLayer1 {
    read_rx: Receiver<Result<Vec<u8>, io::Error>>,
    write_tx: Sender<Vec<u8>>,
}
impl NetClientLayer1 {
    pub async fn new() -> Self {
        let stream = TcpStream::connect("127.0.0.1:5555")
            .await
            .expect("Unable to connect to socket.");

        let (read_tx, read_rx) = tokio::sync::mpsc::channel(1024);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(1024);

        let (read_half, write_half) = stream.into_split();

        tokio::spawn(async move {
            let mut writer = BufWriter::new(write_half);
            loop {
                let bytes = write_rx.recv().await;
                match write(&mut writer, bytes.unwrap()).await {
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("Error in writing to socket: {}", error);
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            loop {
                let response = read(&mut reader).await;
                match response {
                    Ok(bytes) => match read_tx.send(Ok(bytes)).await {
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("Error in channeling read: {}", error);
                            panic!();
                        }
                    },
                    Err(error) => {
                        match read_tx.send(Err(error)).await {
                            Ok(_) => {}
                            Err(error) => {
                                eprintln!("Error in channeling read error: {}", error);
                                panic!();
                            }
                        }
                        break;
                    }
                }
            }
        });

        Self { read_rx, write_tx }
    }
    pub fn dequeue(&mut self) -> Vec<Result<Vec<u8>, io::Error>> {
        let mut messages = Vec::new();
        while let Ok(message) = self.read_rx.try_recv() {
            messages.push(message);
        }
        messages
    }
    pub async fn enqueue(&mut self, bytes: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.write_tx.send(bytes).await
    }
}
