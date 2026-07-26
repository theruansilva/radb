use crate::constants::*;
use crate::error::{AdbError, Result};
use crate::keypair::AdbKeyPair;
use crate::message_queue::MessageQueue;
use crate::reader::AdbReader;
use crate::stream::AdbStream;
use crate::writer::AdbWriter;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

pub struct AdbConnection<W> {
    writer: Arc<Mutex<AdbWriter<W>>>,
    message_queue: MessageQueue,
    next_local_id: AtomicU32,
    pub supported_features: HashSet<String>,
    pub version: u32,
    pub max_payload_size: usize,
}

impl<W: AsyncWrite + Unpin + Send + 'static> AdbConnection<W> {
    pub async fn connect<R: AsyncRead + Unpin + Send + 'static>(
        mut reader: AdbReader<R>,
        mut writer: AdbWriter<W>,
        key_pair: Option<&AdbKeyPair>,
    ) -> Result<Self> {
        // Step 1: Send initial A_CNXN packet
        writer.write_connect().await?;

        // Step 2: Read initial device packet (expecting CNXN or AUTH)
        let mut msg = reader.read_message().await?;

        if msg.command == CMD_AUTH && msg.arg0 == AUTH_TYPE_TOKEN {
            let key_pair = key_pair.ok_or_else(|| {
                AdbError::Auth("Device requires authentication but no key pair was provided".into())
            })?;

            // Try RSA signature auth
            let signature = key_pair.sign_payload(&msg.payload)?;
            writer.write_auth(AUTH_TYPE_SIGNATURE, signature).await?;
            msg = reader.read_message().await?;

            // If device still requests AUTH, send RSA public key
            if msg.command == CMD_AUTH && msg.arg0 == AUTH_TYPE_TOKEN {
                writer
                    .write_auth(AUTH_TYPE_RSA_PUBLIC, key_pair.public_key_bytes.clone())
                    .await?;
                msg = reader.read_message().await?;
            }
        }

        if msg.command != CMD_CNXN {
            return Err(AdbError::Connect(format!(
                "Handshake failed: expected CNXN packet, got {}",
                crate::message::AdbMessage::command_name(msg.command)
            )));
        }

        let version = msg.arg0;
        let max_payload_size = msg.arg1 as usize;
        let banner_str = String::from_utf8_lossy(&msg.payload);
        let supported_features = Self::parse_features(&banner_str)?;
        let writer = Arc::new(Mutex::new(writer));
        let message_queue = MessageQueue::new();
        message_queue.spawn_reader(reader);

        Ok(Self {
            writer,
            message_queue,
            next_local_id: AtomicU32::new(1),
            supported_features,
            version,
            max_payload_size,
        })
    }

    pub async fn open(&self, destination: &str) -> Result<AdbStream<W>> {
        let local_id = self.next_local_id.fetch_add(1, Ordering::SeqCst);
        let mut rx = self.message_queue.start_listening(local_id).await?;

        {
            let mut writer_guard = self.writer.lock().await;
            writer_guard.write_open(local_id, destination).await?;
        }
        match rx.recv().await {
            Some(Ok(response)) => {
                if response.command == CMD_OKAY {
                    let remote_id = response.arg0;
                    Ok(AdbStream::new(
                        local_id,
                        remote_id,
                        self.writer.clone(),
                        rx,
                        self.max_payload_size,
                    ))
                } else if response.command == CMD_CLSE {
                    self.message_queue.stop_listening(local_id).await;
                    Err(AdbError::StreamOpen {
                        destination: destination.to_string(),
                        reason: "Connection refused by ADB daemon".to_string(),
                    })
                } else {
                    self.message_queue.stop_listening(local_id).await;
                    Err(AdbError::Protocol(format!(
                        "Unexpected response opening stream: {}",
                        crate::message::AdbMessage::command_name(response.command)
                    )))
                }
            }
            Some(Err(err)) => {
                self.message_queue.stop_listening(local_id).await;
                Err(err)
            }
            None => {
                self.message_queue.stop_listening(local_id).await;
                Err(AdbError::ConnectionClosed(
                    "Stream channel closed before open acknowledgement".into(),
                ))
            }
        }
    }

    pub fn supports_feature(&self, feature: &str) -> bool {
        self.supported_features.contains(feature)
    }

    fn parse_features(banner: &str) -> Result<HashSet<String>> {
        let mut features = HashSet::new();

        if let Some((_, params)) = banner.split_once("::") {
            for param in params.split(';') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "features" {
                        for feat in value.split(',') {
                            features.insert(feat.trim().to_string());
                        }
                    }
                }
            }
        }

        Ok(features)
    }
}
