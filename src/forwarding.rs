use crate::Radb;
use crate::error::{AdbError, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub struct TcpForwarder {
    pub local_port: u16,
    pub target_port: u16,
    task_handle: Option<JoinHandle<()>>,
}

impl TcpForwarder {
    pub async fn start<D: Radb + 'static>(
        radb: Arc<D>,
        local_port: u16,
        target_port: u16,
    ) -> Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{local_port}"))
            .await
            .map_err(AdbError::Io)?;
        let actual_port = listener.local_addr()?.port();

        let task_handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((client_socket, peer_addr)) => {
                        debug!("TcpForwarder accepted client from {peer_addr}");
                        let radb_clone = radb.clone();

                        tokio::spawn(async move {
                            let dest = format!("tcp:{target_port}");
                            match radb_clone.open(&dest).await {
                                Ok(adb_stream) => {
                                    let (mut client_read, mut client_write) =
                                        client_socket.into_split();

                                    let adb_stream = Arc::new(Mutex::new(adb_stream));

                                    let stream_write = adb_stream.clone();
                                    let forward_client_to_adb = async move {
                                        let mut buf = [0u8; 8192];
                                        loop {
                                            use tokio::io::AsyncReadExt;
                                            match client_read.read(&mut buf).await {
                                                Ok(0) => break,
                                                Ok(n) => {
                                                    let mut stream = stream_write.lock().await;
                                                    if stream
                                                        .write_payload(&buf[..n])
                                                        .await
                                                        .is_err()
                                                    {
                                                        break;
                                                    }
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                    };

                                    let stream_read = adb_stream.clone();
                                    let forward_adb_to_client = async move {
                                        loop {
                                            let chunk_res = {
                                                let mut stream = stream_read.lock().await;
                                                stream.read_chunk().await
                                            };
                                            match chunk_res {
                                                Ok(chunk) if chunk.is_empty() => break,
                                                Ok(chunk) => {
                                                    use tokio::io::AsyncWriteExt;
                                                    if client_write.write_all(&chunk).await.is_err()
                                                    {
                                                        break;
                                                    }
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                    };

                                    tokio::select! {
                                        _ = forward_client_to_adb => {},
                                        _ = forward_adb_to_client => {},
                                    }
                                }
                                Err(err) => {
                                    error!("TcpForwarder failed opening ADB stream target: {err}");
                                }
                            }
                        });
                    }
                    Err(err) => {
                        error!("TcpForwarder accept loop error: {err}");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            local_port: actual_port,
            target_port,
            task_handle: Some(task_handle),
        })
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for TcpForwarder {
    fn drop(&mut self) {
        self.stop();
    }
}
