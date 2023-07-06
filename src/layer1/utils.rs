use std::io::ErrorKind;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

// pub async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
// where
//     T: AsyncRead + Unpin,
// {
//     // let header = reader.read_u8().await.expect("Unable to read header.");
//     // println!("Header: {header}");
//     let mut buf = vec![0u8; 64];
//     let bytes_count = reader.read(&mut buf).await?;
//     if bytes_count == 0 {
//         return Err(io::Error::new(ErrorKind::Other, "Read 0 Bytes!"));
//     }
//     Ok(buf[..bytes_count].to_vec())
// }

pub async fn write<T>(writer: &mut BufWriter<T>, bytes: Vec<u8>) -> Result<(), io::Error>
where
    T: AsyncWrite + Unpin,
{
    // println!("Started write");
    writer
        .write_u32(bytes.len() as u32)
        .await
        .expect("Unable to write header.");
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    // println!("Finished write");
    Ok(())
}

enum ReadState {
    Length,
    Payload,
}

pub async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
where
    T: AsyncRead + Unpin,
{
    // println!("Started read");
    let length = reader.read_u32().await.expect("Unable to read header.");
    let mut buffer = vec![0u8; length as usize];
    reader.read_exact(&mut buffer).await.expect("Unable to read body.");
    // println!("Finished read");
    Ok(buffer)

}

// pub async fn read<T>(reader: &mut BufReader<T>) -> Result<Vec<u8>, io::Error>
// where
//     T: AsyncRead + Unpin,
// {
//     println!("Started read");
//
//     let mut read_state = ReadState::Length;
//
//     let mut length_bytes_read = 0;
//     let mut length_bytes = [0u8; 4];
//
//     let mut payload_bytes_to_read = 0;
//     let mut payload_bytes_read = 0;
//     let mut payload_bytes = [0u8; 1024 * 4]; // 4Kb limit
//
//     let mut buffer = [0u8; 1024];
//
//     loop {
//         let read_result = reader.read(&mut buffer).await;
//
//         let bytes_read = match read_result {
//             Ok(bytes) => bytes,
//             Err(error) => {
//                 // match error.kind() {}
//                 // TODO: Check the error and do something specific
//                 eprintln!("Layer1: Read Error: {:?}", error);
//
//                 return Err(error);
//             }
//         };
//         // let bytes_read = reader.read(&mut buffer).unwrap();
//
//         for i in 0..bytes_read {
//             let byte_read = buffer[i];
//             match read_state {
//                 ReadState::Length => {
//                     length_bytes[length_bytes_read] = byte_read;
//                     length_bytes_read += 1;
//                     if length_bytes_read == 4 {
//                         length_bytes_read = 0;
//                         payload_bytes_to_read = u32::from_be_bytes(length_bytes);
//                         read_state = ReadState::Payload;
//                         if payload_bytes_to_read > payload_bytes.len() as u32 {
//                             eprintln!("Layer1: Buffer: {:?}", buffer.to_vec());
//                             eprintln!("Layer1: Length Bytes: {:?}", length_bytes.to_vec());
//                             eprintln!(
//                                 "Layer1: Length of message too long at {} bytes.",
//                                 payload_bytes_to_read
//                             );
//                         }
//                         println!("Going to read {} bytes.", payload_bytes_to_read);
//                     }
//                 }
//                 ReadState::Payload => {
//                     payload_bytes[payload_bytes_read as usize] = byte_read;
//                     payload_bytes_read += 1;
//                     if payload_bytes_read == payload_bytes_to_read {
//                         let payload = payload_bytes[0..payload_bytes_read as usize].to_vec();
//                         println!("Finished read");
//                         return Ok(payload);
//                     }
//                 }
//             }
//         }
//     }
// }
