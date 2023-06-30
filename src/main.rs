use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
};

async fn server() {
    let tcp_listener = TcpListener::bind("127.0.0.1:5555")
        .await
        .expect("Unable to bind listener to socket.");

    loop {
        println!("Waiting for connection.");
        let (mut stream, _) = tcp_listener
            .accept()
            .await
            .expect("Unable to bind to connecting socket.");

        println!("Connection.");

        tokio::spawn(async move {
            let (read_half, write_half) = stream.split();

            let mut writer = BufWriter::new(write_half);
            let mut reader = BufReader::new(read_half);

            loop {
                let received = read(&mut reader).await;
                // println!("Received: {:?}", String::from_utf8(received).unwrap());
                write(&mut writer, received).await;
            }
        });
    }
}

async fn client() {
    let mut stream = TcpStream::connect("127.0.0.1:5555")
        .await
        .expect("Unable to connect to socket.");

    let (read_half, write_half) = stream.split();

    let mut writer = BufWriter::new(write_half);
    let mut reader = BufReader::new(read_half);

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        write(&mut writer, "Hello from Client.".as_bytes().to_vec()).await;
        let response = read(&mut reader).await;
        println!("Response: {:?}", String::from_utf8(response).unwrap());
    }
}

async fn read<T>(reader: &mut BufReader<T>) -> Vec<u8>
where
    T: AsyncRead + Unpin,
{
    let header = reader.read_u8().await.expect("Unable to read header.");
    println!("Header: {header}");
    let mut buf = vec![0u8; 64];
    reader.read(&mut buf).await.expect("Unable to read header.");
    buf
}

async fn write<T>(writer: &mut BufWriter<T>, bytes: Vec<u8>)
where
    T: AsyncWrite + Unpin,
{
    writer.write_u8(42).await.expect("Unable to write header.");
    writer.write_all(&bytes).await.expect("Could not write.");
    writer.flush().await.unwrap();
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
