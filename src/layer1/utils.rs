use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

pub async fn write<T>(writer: &mut BufWriter<T>, bytes: Vec<u8>) -> Result<(), io::Error>
where
    T: AsyncWrite + Unpin,
{
    writer.write_u32(bytes.len() as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
where
    T: AsyncRead + Unpin,
{
    let length = reader.read_u32().await?;
    let mut buffer = vec![0u8; length as usize];
    reader.read_exact(&mut buffer).await?;
    Ok(buffer)
}
