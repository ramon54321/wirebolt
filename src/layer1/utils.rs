use std::io::ErrorKind;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

pub async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
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
    Ok(buf[..bytes_count].to_vec())
}

pub async fn write<T>(writer: &mut BufWriter<T>, bytes: Vec<u8>) -> Result<(), io::Error>
where
    T: AsyncWrite + Unpin,
{
    // writer.write_u8(42).await.expect("Unable to write header.");
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}
