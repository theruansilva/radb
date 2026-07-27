#![warn(missing_docs)]
//! # RADB - Rust Android Debug Bridge
//!
//! `radb` is a pure-Rust, asynchronous client implementation of the Android Debug Bridge (ADB) protocol.
//! It provides high-level primitives for interacting with Android devices and emulators directly via TCP,
//! eliminating the need for an external `adb` binary server in many scenarios.
//!
//! ## Key Features
//!
//! - **Direct Protocol Connection**: Connect directly to ADB daemon over TCP (e.g. `127.0.0.1:5555`).
//! - **Automatic Authentication**: Built-in RSA key generation and loading (`~/.android/adbkey`).
//! - **Emulator Discovery**: Discover and list active emulators automatically.
//! - **Shell Execution**: Shell v1 and Shell v2 support with stdout, stderr, and exit codes.
//! - **Touch & Gesture Simulation**: Tap, double tap, long press, swipe, drag & drop, key events, and typing.
//! - **File Synchronization**: Fast `push` and `pull` using ADB Sync protocol.

/// ADB connection management and session handling.
pub mod connection;
/// Protocol constants (commands, ports, timeouts, shell stream IDs).
pub mod constants;
/// Custom error and result types for RADB operations.
pub mod error;
/// TCP port forwarding from host to Android device.
pub mod forwarding;
/// RSA keypair generation, loading (`~/.android/adbkey`), and ADB token signing.
pub mod keypair;
/// Wire protocol ADB message structures and checksum calculation.
pub mod message;
/// AdbReader utility for deserializing ADB packets from an async stream.
pub mod message_queue;
/// Asynchronous message queue and reader background task dispatching.
pub mod reader;
/// Local ADB server (port 5037) communication and process management.
pub mod server;
/// Shell v1 and Shell v2 command execution helpers.
pub mod shell;
/// Stream multiplexing layer for reading and writing data over open ADB channels.
pub mod stream;
/// File synchronization service (push, pull, push_bytes, pull_bytes).
pub mod sync;
/// High-level touch, swipe, gesture, and input event simulation extensions.
pub mod touch;
/// AdbWriter utility for serializing ADB packets to an async stream.
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

/// Default transport stream type for active TCP ADB connections.
pub type TransportStream = AdbStream<OwnedWriteHalf>;

/// Core trait providing standard ADB operations for an Android device or emulator.
#[async_trait]
pub trait Radb: Send + Sync {
    /// Opens a custom raw ADB stream to a specified destination service (e.g. `"shell,v2,raw:..."`, `"sync:"`, `"tcp:8080"`).
    async fn open(&self, destination: &str) -> Result<TransportStream>;

    /// Checks whether the connected device advertises support for a specific feature (e.g., `"shell_v2"`, `"cmd"`).
    fn supports_feature(&self, feature: &str) -> bool;

    /// Executes a shell command on the remote device, automatically selecting `shell_v2` if supported.
    async fn shell(&self, command: &str) -> Result<AdbShellResponse>;

    /// Pushes a local file to the remote device filesystem via ADB Sync.
    async fn push(
        &self,
        src: &Path,
        remote_path: &str,
        mode: u32,
        last_modified_ms: u64,
    ) -> Result<SyncResult>;

    /// Pulls a remote file from the device to the local filesystem via ADB Sync.
    async fn pull(&self, dst: &Path, remote_path: &str) -> Result<SyncResult>;

    /// Installs an APK file onto the remote device using Android's package manager.
    async fn install(&self, apk_path: &Path, options: &[&str]) -> Result<InstallResult>;

    /// Uninstalls a package by name from the remote device.
    async fn uninstall(&self, package_name: &str) -> Result<UninstallResult>;

    /// Restarts the ADB daemon on the target device with root privileges.
    async fn root(&self) -> Result<RootResult>;

    /// Restarts the ADB daemon on the target device without root privileges.
    async fn unroot(&self) -> Result<RootResult>;
}

/// Primary implementation of the `Radb` trait for direct TCP ADB connections.
pub struct RadbImpl {
    host: String,
    port: u16,
    key_pair: Option<Arc<AdbKeyPair>>,
    connection: Arc<Mutex<Option<AdbConnection<OwnedWriteHalf>>>>,
}

impl RadbImpl {
    /// Connects directly to an ADB device at `host:port` using a specified keypair, or the default `~/.android/adbkey`.
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

    /// Scans standard emulator ports (5555..5683) on `host` and returns a client for the first responsive device.
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

    /// Scans standard emulator ports (5555..5683) on `host` and returns clients for all active devices found.
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
