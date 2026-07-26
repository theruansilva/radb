use crate::constants::*;
use crate::error::Result;
use crate::message::AdbMessage;
use bytes::Bytes;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct AdbWriter<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> AdbWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub async fn write_message(&mut self, message: &AdbMessage) -> Result<()> {
        self.writer.write_u32_le(message.command).await?;
        self.writer.write_u32_le(message.arg0).await?;
        self.writer.write_u32_le(message.arg1).await?;
        self.writer.write_u32_le(message.payload_length).await?;
        self.writer.write_u32_le(message.checksum).await?;
        self.writer.write_u32_le(message.magic).await?;

        if !message.payload.is_empty() {
            self.writer.write_all(&message.payload).await?;
        }

        self.writer.flush().await?;
        Ok(())
    }

    pub async fn write_connect(&mut self) -> Result<()> {
        let message = AdbMessage::new(
            CMD_CNXN,
            CONNECT_VERSION,
            CONNECT_MAXDATA,
            Bytes::from_static(CONNECT_PAYLOAD),
        );
        self.write_message(&message).await
    }

    pub async fn write_auth(&mut self, auth_type: u32, auth_payload: impl Into<Bytes>) -> Result<()> {
        let message = AdbMessage::new(CMD_AUTH, auth_type, 0, auth_payload);
        self.write_message(&message).await
    }

    pub async fn write_open(&mut self, local_id: u32, destination: &str) -> Result<()> {
        let mut payload = Vec::with_capacity(destination.len() + 1);
        payload.extend_from_slice(destination.as_bytes());
        payload.push(0); // Null terminator required by ADB protocol
        let message = AdbMessage::new(CMD_OPEN, local_id, 0, payload);
        self.write_message(&message).await
    }

    pub async fn write_write(&mut self, local_id: u32, remote_id: u32, payload: impl Into<Bytes>) -> Result<()> {
        let message = AdbMessage::new(CMD_WRTE, local_id, remote_id, payload);
        self.write_message(&message).await
    }

    pub async fn write_okay(&mut self, local_id: u32, remote_id: u32) -> Result<()> {
        let message = AdbMessage::new(CMD_OKAY, local_id, remote_id, Bytes::new());
        self.write_message(&message).await
    }

    pub async fn write_close(&mut self, local_id: u32, remote_id: u32) -> Result<()> {
        let message = AdbMessage::new(CMD_CLSE, local_id, remote_id, Bytes::new());
        self.write_message(&message).await
    }
}
