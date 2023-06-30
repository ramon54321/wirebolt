use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
};

async fn server() {
    let tcp_listener = TcpListener::bind("127.0.0.1:5555")
        .await
        .expect("Unable to bind listener to socket.");

    loop {
        let (mut stream, _) = tcp_listener
            .accept()
            .await
            .expect("Unable to bind to connecting socket.");

        println!("Connection.");

        let (read_half, write_half) = stream.split();

        let mut writer = BufWriter::new(write_half);
        let mut reader = BufReader::new(read_half);

        write(&mut writer, "Hello from Server.".as_bytes().to_vec()).await;

        let response = read(&mut reader).await;
        println!("Response: {:?}", String::from_utf8(response).unwrap());
    }
}

async fn client() {
    let mut stream = TcpStream::connect("127.0.0.1:5555")
        .await
        .expect("Unable to connect to socket.");

    let (read_half, write_half) = stream.split();

    let mut writer = BufWriter::new(write_half);
    let mut reader = BufReader::new(read_half);

    let response = read(&mut reader).await;
    println!("Response: {:?}", String::from_utf8(response).unwrap());

    write(&mut writer, "Hello from Client.".as_bytes().to_vec()).await;
}

async fn read<T>(reader: &mut BufReader<T>) -> Vec<u8>
where
    T: AsyncRead + Unpin,
{
    let header = reader.read_u8().await.expect("Unable to read header.");
    println!("Header: {header}");
    let mut buf = vec![0u8; 16];
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
