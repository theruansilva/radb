use crate::error::{AdbError, Result};
use crate::message::AdbMessage;
use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct AdbReader<R> {
    reader: R,
}

impl<R: AsyncRead + Unpin> AdbReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub async fn read_message(&mut self) -> Result<AdbMessage> {
        let command = match self.reader.read_u32_le().await {
            Ok(cmd) => cmd,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(AdbError::ConnectionClosed("EOF reading command header".into()));
            }
            Err(e) => return Err(AdbError::Io(e)),
        };

        let arg0 = self.reader.read_u32_le().await?;
        let arg1 = self.reader.read_u32_le().await?;
        let payload_length = self.reader.read_u32_le().await?;
        let checksum = self.reader.read_u32_le().await?;
        let magic = self.reader.read_u32_le().await?;

        let payload = if payload_length > 0 {
            let mut buf = BytesMut::zeroed(payload_length as usize);
            self.reader.read_exact(&mut buf).await?;
            buf.freeze()
        } else {
            bytes::Bytes::new()
        };

        Ok(AdbMessage {
            command,
            arg0,
            arg1,
            payload_length,
            checksum,
            magic,
            payload,
        })
    }
}
