use crate::constants::{CMD_CLSE, CMD_WRTE};
use crate::error::{AdbError, Result};
use crate::message::AdbMessage;
use crate::writer::AdbWriter;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tokio::io::AsyncWrite;
use tokio::sync::{mpsc, Mutex};

pub struct AdbStream<W> {
    pub local_id: u32,
    pub remote_id: u32,
    writer: Arc<Mutex<AdbWriter<W>>>,
    rx: mpsc::Receiver<Result<AdbMessage>>,
    max_payload_size: usize,
    read_buf: BytesMut,
    is_closed: bool,
}

impl<W: AsyncWrite + Unpin + Send + 'static> AdbStream<W> {
    pub fn new(
        local_id: u32,
        remote_id: u32,
        writer: Arc<Mutex<AdbWriter<W>>>,
        rx: mpsc::Receiver<Result<AdbMessage>>,
        max_payload_size: usize,
    ) -> Self {
        Self {
            local_id,
            remote_id,
            writer,
            rx,
            max_payload_size,
            read_buf: BytesMut::new(),
            is_closed: false,
        }
    }

    /// Read next raw data chunk from device (returns empty Bytes on EOF/Close)
    pub async fn read_chunk(&mut self) -> Result<Bytes> {
        if self.is_closed {
            return Ok(Bytes::new());
        }

        if !self.read_buf.is_empty() {
            let data = self.read_buf.split().freeze();
            return Ok(data);
        }

        match self.rx.recv().await {
            Some(Ok(msg)) => {
                if msg.command == CMD_CLSE {
                    self.is_closed = true;
                    Ok(Bytes::new())
                } else if msg.command == CMD_WRTE {
                    let payload = msg.payload;
                    // Auto-send A_OKAY flow-control response
                    {
                        let mut writer_guard = self.writer.lock().await;
                        writer_guard.write_okay(self.local_id, self.remote_id).await?;
                    }
                    Ok(payload)
                } else {
                    Err(AdbError::Protocol(format!(
                        "Unexpected ADB command in stream: {}",
                        AdbMessage::command_name(msg.command)
                    )))
                }
            }
            Some(Err(err)) => {
                self.is_closed = true;
                Err(err)
            }
            None => {
                self.is_closed = true;
                Ok(Bytes::new())
            }
        }
    }

    /// Write raw payload to device in max_payload_size chunks
    pub async fn write_payload(&mut self, payload: &[u8]) -> Result<()> {
        if self.is_closed {
            return Err(AdbError::StreamClosed(self.local_id));
        }

        for chunk in payload.chunks(self.max_payload_size) {
            let mut writer_guard = self.writer.lock().await;
            writer_guard
                .write_write(self.local_id, self.remote_id, Bytes::copy_from_slice(chunk))
                .await?;
        }

        Ok(())
    }

    /// Close the stream sending A_CLSE packet
    pub async fn close(&mut self) -> Result<()> {
        if !self.is_closed {
            self.is_closed = true;
            let mut writer_guard = self.writer.lock().await;
            writer_guard.write_close(self.local_id, self.remote_id).await?;
        }
        Ok(())
    }
}
