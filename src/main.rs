use std::{collections::HashMap, io::ErrorKind, sync::Arc, time::Duration};

use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    select,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

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
                    .map(|message| String::from_utf8(message.1.clone()).unwrap())
                    .collect::<Vec<_>>()
            );
        }

        let name = {
            let write_txs = net_server.write_txs.lock().await;
            let keys = write_txs.keys().collect::<Vec<_>>();
            if keys.is_empty() {
                None
            } else {
                let index = (count % keys.len());
                println!("{index}");
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

struct NetServerLayer1 {
    read_rxs: Arc<Mutex<HashMap<String, Receiver<Vec<u8>>>>>,
    write_txs: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
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
                    println!("Waiting for connection.");
                    let (mut stream, address) = tcp_listener
                        .accept()
                        .await
                        .expect("Unable to bind to connecting socket.");
                    let name = address.to_string();
                    println!("Connection from {}.", name.clone());

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
                                // let received = read(&mut reader).await;
                                // println!("Received: {:?}", String::from_utf8(received).unwrap());
                                // tokio::time::sleep(Duration::from_millis(500)).await;
                                // write(&mut writer, b"Greetings...".to_vec()).await;

                                select! {
                                    bytes = write_rx.recv() => {
                                        match write(&mut writer, bytes.unwrap()).await {
                                            Ok(_) => {},
                                            Err(error) => eprintln!("Error in writing to socket: {}", error),
                                        }
                                    }
                                    response = read(&mut reader) => {
                                        // println!("Response: {:?}", String::from_utf8(response.clone()).unwrap());
                                        // read_tx.try_send(response).unwrap();
                                        match response {
                                            Ok(bytes) => {
                                                println!("Got byts: {:?}", bytes);
                                                match read_tx.try_send(bytes) {
                                                    Ok(_) => {},
                                                    Err(error) => {
                                                        eprintln!("Error in channeling read: {}", error);
                                                        panic!();
                                                    }
                                                }
                                            },
                                            Err(error) => {
                                                eprintln!("Error in read: {}", error);
                                                println!("Removing this client handler");
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
    pub async fn dequeue(&mut self) -> Vec<(String, Vec<u8>)> {
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

struct NetClientLayer1 {
    read_rx: Receiver<Vec<u8>>,
    write_tx: Sender<Vec<u8>>,
}
impl NetClientLayer1 {
    pub async fn new() -> Self {
        let mut stream = TcpStream::connect("127.0.0.1:5555")
            .await
            .expect("Unable to connect to socket.");

        let (read_tx, read_rx) = tokio::sync::mpsc::channel(64);
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            let (read_half, write_half) = stream.split();

            let mut writer = BufWriter::new(write_half);
            let mut reader = BufReader::new(read_half);

            loop {
                // tokio::time::sleep(Duration::from_millis(500)).await;
                // write(&mut writer, "Hello from Client.".as_bytes().to_vec()).await;

                select! {
                    bytes = write_rx.recv() => {
                        // write(&mut writer, bytes.unwrap()).await;
                        match write(&mut writer, bytes.unwrap()).await {
                            Ok(_) => {},
                            Err(error) => eprintln!("Error in writing to socket: {}", error),
                        }
                    }
                    response = read(&mut reader) => {
                        // println!("Response: {:?}", String::from_utf8(response.clone()).unwrap());
                        match response {
                            Ok(bytes) => {
                                read_tx.try_send(bytes).unwrap();
                            },
                            Err(error) => {
                                eprintln!("Error in read: {}", error);
                            },
                        }
                    }
                }

                // println!("After select");
            }
        });

        Self { read_rx, write_tx }
    }
    pub fn dequeue(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        while let Ok(message) = self.read_rx.try_recv() {
            messages.push(message);
        }
        messages
    }
    pub fn enqueue(&mut self, bytes: Vec<u8>) {
        self.write_tx.try_send(bytes).unwrap();
    }
}

async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
where
    T: AsyncRead + Unpin,
{
    // let header = reader.read_u8().await.expect("Unable to read header.");
    // println!("Header: {header}");
    let mut buf = vec![0u8; 64];
    let bytes_count = reader.read(&mut buf).await?;
    if bytes_count == 0 {
        return Err(io::Error::new(ErrorKind::Other, "Read 0 Bytes!"));
    }
    Ok(buf)
}

async fn write<T>(writer: &mut BufWriter<T>, bytes: Vec<u8>) -> Result<(), io::Error>
where
    T: AsyncWrite + Unpin,
{
    // writer.write_u8(42).await.expect("Unable to write header.");
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
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

    // println!("After client created.");
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
