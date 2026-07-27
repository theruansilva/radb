pub mod connection;
pub mod constants;
pub mod error;
pub mod forwarding;
pub mod keypair;
pub mod message;
pub mod message_queue;
pub mod reader;
pub mod server;
pub mod shell;
pub mod stream;
pub mod sync;
pub mod touch;
pub mod writer;

use async_trait::async_trait;
use connection::AdbConnection;
use constants::{MAX_EMULATOR_PORT, MIN_EMULATOR_PORT};
use error::{AdbError, InstallResult, Result, RootResult, SyncResult, UninstallResult};
use keypair::AdbKeyPair;
use reader::AdbReader;
use shell::{AdbShellResponse, execute_shell};
use std::path::Path;
use std::sync::Arc;
use stream::AdbStream;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
pub use touch::{KeyCode, RadbTouchExt, SwipeDirection};
use writer::AdbWriter;

pub type TransportStream = AdbStream<OwnedWriteHalf>;

#[async_trait]
pub trait Radb: Send + Sync {
    async fn open(&self, destination: &str) -> Result<TransportStream>;
    fn supports_feature(&self, feature: &str) -> bool;
    async fn shell(&self, command: &str) -> Result<AdbShellResponse>;
    async fn push(
        &self,
        src: &Path,
        remote_path: &str,
        mode: u32,
        last_modified_ms: u64,
    ) -> Result<SyncResult>;
    async fn pull(&self, dst: &Path, remote_path: &str) -> Result<SyncResult>;
    async fn install(&self, apk_path: &Path, options: &[&str]) -> Result<InstallResult>;
    async fn uninstall(&self, package_name: &str) -> Result<UninstallResult>;
    async fn root(&self) -> Result<RootResult>;
    async fn unroot(&self) -> Result<RootResult>;
}

pub struct RadbImpl {
    host: String,
    port: u16,
    key_pair: Option<Arc<AdbKeyPair>>,
    connection: Arc<Mutex<Option<AdbConnection<OwnedWriteHalf>>>>,
}

impl RadbImpl {
    pub async fn connect(host: &str, port: u16, key_pair: Option<AdbKeyPair>) -> Result<Self> {
        let key_pair = key_pair
            .or_else(|| AdbKeyPair::read_default().ok())
            .map(Arc::new);
        let radb = Self {
            host: host.to_string(),
            port,
            key_pair,
            connection: Arc::new(Mutex::new(None)),
        };
        radb.ensure_connected().await?;
        Ok(radb)
    }

    pub async fn discover(host: &str, key_pair: Option<AdbKeyPair>) -> Result<Option<Self>> {
        let key_pair = key_pair.or_else(|| AdbKeyPair::read_default().ok());
        for port in (MIN_EMULATOR_PORT..=MAX_EMULATOR_PORT).step_by(2) {
            if let Ok(radb) = Self::connect(host, port, key_pair.clone()).await {
                if radb.shell("echo ping").await.is_ok() {
                    return Ok(Some(radb));
                }
            }
        }
        Ok(None)
    }

    pub async fn list(host: &str, key_pair: Option<AdbKeyPair>) -> Result<Vec<Self>> {
        let key_pair = key_pair.or_else(|| AdbKeyPair::read_default().ok());
        let mut devices = Vec::new();
        for port in (MIN_EMULATOR_PORT..=MAX_EMULATOR_PORT).step_by(2) {
            if let Ok(radb) = Self::connect(host, port, key_pair.clone()).await {
                if radb.shell("echo ping").await.is_ok() {
                    devices.push(radb);
                }
            }
        }
        Ok(devices)
    }

    async fn ensure_connected(&self) -> Result<Arc<Mutex<Option<AdbConnection<OwnedWriteHalf>>>>> {
        let mut conn_guard = self.connection.lock().await;
        if conn_guard.is_none() {
            let addr = format!("{}:{}", self.host, self.port);
            let socket = TcpStream::connect(&addr).await?;
            let (read_half, write_half) = socket.into_split();

            let reader = AdbReader::new(read_half);
            let writer = AdbWriter::new(write_half);

            let connection =
                AdbConnection::connect(reader, writer, self.key_pair.as_deref()).await?;
            *conn_guard = Some(connection);
        }
        drop(conn_guard);
        Ok(self.connection.clone())
    }
}

/// Connect to an ADB device using the default keypair (`~/.android/adbkey`).
pub async fn connect(host: &str, port: u16) -> Result<RadbImpl> {
    RadbImpl::connect(host, port, None).await
}

/// Discover the first available emulator using the default keypair (`~/.android/adbkey`).
pub async fn discover(host: &str) -> Result<Option<RadbImpl>> {
    RadbImpl::discover(host, None).await
}

/// List all available emulators using the default keypair (`~/.android/adbkey`).
pub async fn list(host: &str) -> Result<Vec<RadbImpl>> {
    RadbImpl::list(host, None).await
}

#[async_trait]
impl Radb for RadbImpl {
    async fn open(&self, destination: &str) -> Result<TransportStream> {
        let conn_arc = self.ensure_connected().await?;
        let conn_guard = conn_arc.lock().await;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| AdbError::ConnectionClosed("Connection was not initialized".into()))?;
        conn.open(destination).await
    }

    fn supports_feature(&self, feature: &str) -> bool {
        if let Ok(guard) = self.connection.try_lock() {
            if let Some(conn) = guard.as_ref() {
                return conn.supports_feature(feature);
            }
        }
        false
    }

    async fn shell(&self, command: &str) -> Result<AdbShellResponse> {
        self.ensure_connected().await?;
        let is_v2 = self.supports_feature("shell_v2");
        let dest = if is_v2 {
            format!("shell,v2,raw:{command}")
        } else {
            format!("shell,raw:{command}")
        };

        let mut stream = self.open(&dest).await?;
        execute_shell(&mut stream, is_v2).await
    }

    async fn push(
        &self,
        src: &Path,
        remote_path: &str,
        mode: u32,
        last_modified_ms: u64,
    ) -> Result<SyncResult> {
        let mut stream = self.open("sync:").await?;
        sync::push(&mut stream, src, remote_path, mode, last_modified_ms).await
    }

    async fn pull(&self, dst: &Path, remote_path: &str) -> Result<SyncResult> {
        let mut stream = self.open("sync:").await?;
        sync::pull(&mut stream, dst, remote_path).await
    }

    async fn install(&self, apk_path: &Path, options: &[&str]) -> Result<InstallResult> {
        let file_size = tokio::fs::metadata(apk_path).await?.len();
        let opts = if options.is_empty() {
            String::new()
        } else {
            format!(" {}", options.join(" "))
        };
        let dest = format!("exec:cmd package install -S {file_size}{opts}");

        let mut stream = self.open(&dest).await?;
        let apk_bytes = tokio::fs::read(apk_path).await?;
        stream.write_payload(&apk_bytes).await?;

        let response = execute_shell(&mut stream, false).await?;

        if response.output.contains("Success") {
            Ok(InstallResult::Success)
        } else {
            Ok(InstallResult::Failure(response.output))
        }
    }

    async fn uninstall(&self, package_name: &str) -> Result<UninstallResult> {
        let response = self
            .shell(&format!("cmd package uninstall {package_name}"))
            .await?;
        if response.output.contains("Success") {
            Ok(UninstallResult::Success)
        } else {
            Ok(UninstallResult::Failure {
                reason: response.output,
                exit_code: response.exit_code,
            })
        }
    }

    async fn root(&self) -> Result<RootResult> {
        let mut stream = self.open("root:").await?;
        let response = execute_shell(&mut stream, false).await?;
        if response.output.contains("restarting adbd as root")
            || response.output.contains("already running as root")
        {
            Ok(RootResult::Success)
        } else {
            Ok(RootResult::Failure(response.output))
        }
    }

    async fn unroot(&self) -> Result<RootResult> {
        let mut stream = self.open("unroot:").await?;
        let response = execute_shell(&mut stream, false).await?;
        if response.output.contains("restarting adbd as non-root") {
            Ok(RootResult::Success)
        } else {
            Ok(RootResult::Failure(response.output))
        }
    }
}
