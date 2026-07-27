use crate::error::{AdbError, Result, SyncResult};
use crate::stream::AdbStream;
use bytes::{Buf, BytesMut};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const SYNC_CHUNK_SIZE: usize = 64 * 1024; // 64 KB

pub const ID_SEND: [u8; 4] = *b"SEND";
pub const ID_RECV: [u8; 4] = *b"RECV";
pub const ID_DATA: [u8; 4] = *b"DATA";
pub const ID_DONE: [u8; 4] = *b"DONE";
pub const ID_OKAY: [u8; 4] = *b"OKAY";
pub const ID_FAIL: [u8; 4] = *b"FAIL";
pub const ID_QUIT: [u8; 4] = *b"QUIT";

pub async fn push<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
    src_path: impl AsRef<Path>,
    remote_path: &str,
    mode: u32,
    last_modified_ms: u64,
) -> Result<SyncResult> {
    let mut file = File::open(src_path).await?;
    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data).await?;

    push_bytes(stream, &file_data, remote_path, mode, last_modified_ms).await
}

pub async fn push_bytes<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
    data: &[u8],
    remote_path: &str,
    mode: u32,
    last_modified_ms: u64,
) -> Result<SyncResult> {
    // 1. Send SEND packet: SEND + len(remote_path,mode)
    let destination_spec = format!("{remote_path},{mode}");
    let mut send_packet = Vec::with_capacity(8 + destination_spec.len());
    send_packet.extend_from_slice(&ID_SEND);
    send_packet.extend_from_slice(&(destination_spec.len() as u32).to_le_bytes());
    send_packet.extend_from_slice(destination_spec.as_bytes());

    stream.write_payload(&send_packet).await?;

    // 2. Send DATA chunks
    for chunk in data.chunks(SYNC_CHUNK_SIZE) {
        let mut data_packet = Vec::with_capacity(8 + chunk.len());
        data_packet.extend_from_slice(&ID_DATA);
        data_packet.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        data_packet.extend_from_slice(chunk);
        stream.write_payload(&data_packet).await?;
    }

    // 3. Send DONE packet
    let timestamp_sec = (last_modified_ms / 1000) as u32;
    let mut done_packet = Vec::with_capacity(8);
    done_packet.extend_from_slice(&ID_DONE);
    done_packet.extend_from_slice(&timestamp_sec.to_le_bytes());
    stream.write_payload(&done_packet).await?;

    // 4. Read OKAY or FAIL response
    read_response(stream).await
}

pub async fn pull<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
    dst_path: impl AsRef<Path>,
    remote_path: &str,
) -> Result<SyncResult> {
    let pulled_bytes = pull_bytes(stream, remote_path).await?;
    match pulled_bytes {
        Ok(bytes) => {
            let mut file = File::create(dst_path).await?;
            file.write_all(&bytes).await?;
            Ok(SyncResult::Success)
        }
        Err(reason) => Ok(SyncResult::Failure(reason)),
    }
}

pub async fn pull_bytes<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
    remote_path: &str,
) -> Result<std::result::Result<Vec<u8>, String>> {
    // Send RECV packet
    let mut recv_packet = Vec::with_capacity(8 + remote_path.len());
    recv_packet.extend_from_slice(&ID_RECV);
    recv_packet.extend_from_slice(&(remote_path.len() as u32).to_le_bytes());
    recv_packet.extend_from_slice(remote_path.as_bytes());

    stream.write_payload(&recv_packet).await?;

    let mut output_bytes = Vec::new();
    let mut buf = BytesMut::new();

    loop {
        let chunk = stream.read_chunk().await?;
        if chunk.is_empty() && buf.is_empty() {
            break;
        }
        buf.extend_from_slice(&chunk);

        while buf.len() >= 8 {
            let id = [buf[0], buf[1], buf[2], buf[3]];
            let arg = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;

            if id == ID_DONE {
                buf.advance(8);
                return Ok(Ok(output_bytes));
            } else if id == ID_FAIL {
                if buf.len() < 8 + arg {
                    break; // Wait for full error payload
                }
                buf.advance(8);
                let err_bytes = buf.split_to(arg);
                let reason = String::from_utf8_lossy(&err_bytes).to_string();
                return Ok(Err(reason));
            } else if id == ID_DATA {
                if buf.len() < 8 + arg {
                    break; // Wait for full data payload
                }
                buf.advance(8);
                let data_chunk = buf.split_to(arg);
                output_bytes.extend_from_slice(&data_chunk);
            } else {
                return Err(AdbError::Protocol(format!(
                    "Unexpected SYNC packet header: {:?}",
                    String::from_utf8_lossy(&id)
                )));
            }
        }
    }

    Ok(Ok(output_bytes))
}

async fn read_response<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
) -> Result<SyncResult> {
    let mut buf = BytesMut::new();
    loop {
        let chunk = stream.read_chunk().await?;
        if chunk.is_empty() && buf.is_empty() {
            return Err(AdbError::ConnectionClosed(
                "EOF reading SYNC response".into(),
            ));
        }
        buf.extend_from_slice(&chunk);

        if buf.len() >= 8 {
            let id = [buf[0], buf[1], buf[2], buf[3]];
            let arg = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;

            if id == ID_OKAY {
                return Ok(SyncResult::Success);
            } else if id == ID_FAIL {
                if buf.len() >= 8 + arg {
                    buf.advance(8);
                    let err_bytes = buf.split_to(arg);
                    let reason = String::from_utf8_lossy(&err_bytes).to_string();
                    return Ok(SyncResult::Failure(reason));
                }
            } else {
                return Err(AdbError::Protocol(format!(
                    "Unexpected SYNC response ID: {:?}",
                    String::from_utf8_lossy(&id)
                )));
            }
        }
    }
}
