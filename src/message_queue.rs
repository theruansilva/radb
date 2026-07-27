use crate::constants::CMD_CLSE;
use crate::error::{AdbError, Result};
use crate::message::AdbMessage;
use crate::reader::AdbReader;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error};

type StreamSender = mpsc::Sender<Result<AdbMessage>>;
type StreamReceiver = mpsc::Receiver<Result<AdbMessage>>;

#[derive(Clone, Default)]
pub struct MessageQueue {
    streams: Arc<Mutex<HashMap<u32, StreamSender>>>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_listening(&self, local_id: u32) -> Result<StreamReceiver> {
        let mut streams = self.streams.lock().await;
        if streams.contains_key(&local_id) {
            return Err(AdbError::Protocol(format!(
                "Stream local_id {local_id} is already in use"
            )));
        }
        let (tx, rx) = mpsc::channel(128);
        streams.insert(local_id, tx);
        Ok(rx)
    }

    pub async fn stop_listening(&self, local_id: u32) {
        let mut streams = self.streams.lock().await;
        streams.remove(&local_id);
    }

    pub fn spawn_reader<R: AsyncRead + Unpin + Send + 'static>(&self, mut reader: AdbReader<R>) {
        let streams = self.streams.clone();
        tokio::spawn(async move {
            loop {
                match reader.read_message().await {
                    Ok(message) => {
                        let local_id = message.arg1;
                        let is_close = message.command == CMD_CLSE;
                        let sender = {
                            let streams_guard = streams.lock().await;
                            streams_guard.get(&local_id).cloned()
                        };

                        if let Some(tx) = sender {
                            let _ = tx.send(Ok(message)).await;
                            if is_close {
                                let mut streams_guard = streams.lock().await;
                                streams_guard.remove(&local_id);
                            }
                        } else {
                            debug!("Unhandled or orphan ADB message for local_id {local_id}");
                        }
                    }
                    Err(err) => {
                        error!("ADB socket reader loop stopped: {err}");
                        let mut streams_guard = streams.lock().await;
                        let err_msg = err.to_string();
                        for (_id, tx) in streams_guard.drain() {
                            let _ = tx
                                .send(Err(AdbError::ConnectionClosed(err_msg.clone())))
                                .await;
                        }
                        break;
                    }
                }
            }
        });
    }
}
