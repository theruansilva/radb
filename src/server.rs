use crate::error::{AdbError, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Default TCP port for the local ADB host server daemon.
pub const ADB_SERVER_PORT: u16 = 5037;

/// Descriptor representing a device returned by `AdbServer::list_devices()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDeviceDescriptor {
    /// Device serial number or address (e.g. `"emulator-5554"` or `"127.0.0.1:5555"`).
    pub serial: String,
    /// Connection state (e.g. `"device"`, `"offline"`, `"unauthorized"`).
    pub state: String,
}

/// Client for communicating with a local ADB host server daemon (port 5037).
pub struct AdbServer {
    host: String,
    port: u16,
}

impl AdbServer {
    /// Creates a new `AdbServer` instance pointing to the given `host` and `port`.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Creates an `AdbServer` pointing to `127.0.0.1:5037`.
    pub fn default_local() -> Self {
        Self::new("127.0.0.1", ADB_SERVER_PORT)
    }

    /// Checks whether the ADB server daemon is currently running on host:port.
    pub async fn is_running(&self) -> bool {
        TcpStream::connect((self.host.as_str(), self.port))
            .await
            .is_ok()
    }

    /// Ensures the local ADB server daemon is running, attempting to start it if offline.
    pub async fn ensure_server_running(&self) -> Result<()> {
        if self.is_running().await {
            return Ok(());
        }

        AdbBinary::start_server()?;

        // Poll for server startup (up to 5 seconds)
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if self.is_running().await {
                return Ok(());
            }
        }

        Err(AdbError::Connect(
            "Failed to start local ADB server daemon".into(),
        ))
    }

    /// Queries the ADB host server for a list of connected devices (`host:devices`).
    pub async fn list_devices(&self) -> Result<Vec<AdbDeviceDescriptor>> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).await?;
        let output = Self::send_command(&mut stream, "host:devices").await?;

        let mut devices = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((serial, state)) = line.split_once('\t') {
                devices.push(AdbDeviceDescriptor {
                    serial: serial.to_string(),
                    state: state.to_string(),
                });
            }
        }

        Ok(devices)
    }

    /// Sends a length-prefixed command string to an ADB server TCP stream and reads the response.
    pub async fn send_command(stream: &mut TcpStream, command: &str) -> Result<String> {
        let req = format!("{:04x}{command}", command.len());
        stream.write_all(req.as_bytes()).await?;

        let mut status = [0u8; 4];
        stream.read_exact(&mut status).await?;

        if &status == b"OKAY" {
            // Check if there is payload length
            let mut hex_len = [0u8; 4];
            if stream.read_exact(&mut hex_len).await.is_ok() {
                if let Ok(len_str) = std::str::from_utf8(&hex_len) {
                    if let Ok(len) = usize::from_str_radix(len_str, 16) {
                        let mut body = vec![0u8; len];
                        stream.read_exact(&mut body).await?;
                        return Ok(String::from_utf8_lossy(&body).to_string());
                    }
                }
            }
            Ok(String::new())
        } else if &status == b"FAIL" {
            let mut hex_len = [0u8; 4];
            stream.read_exact(&mut hex_len).await?;
            let len_str = std::str::from_utf8(&hex_len)
                .map_err(|e| AdbError::Protocol(format!("Invalid FAIL hex length: {e}")))?;
            let len = usize::from_str_radix(len_str, 16)
                .map_err(|e| AdbError::Protocol(format!("Invalid FAIL hex number: {e}")))?;

            let mut err_body = vec![0u8; len];
            stream.read_exact(&mut err_body).await?;
            let reason = String::from_utf8_lossy(&err_body).to_string();

            Err(AdbError::Protocol(format!(
                "ADB server command failed: {reason}"
            )))
        } else {
            Err(AdbError::Protocol(format!(
                "Unexpected status from ADB server: {:?}",
                String::from_utf8_lossy(&status)
            )))
        }
    }
}

/// Utilities for locating and executing the system `adb` command-line binary.
pub struct AdbBinary;

impl AdbBinary {
    /// Searches `PATH`, `ANDROID_HOME`, and `ANDROID_SDK_ROOT` for the `adb` executable.
    pub fn find_adb_binary() -> Result<PathBuf> {
        if Command::new("adb").arg("version").output().is_ok() {
            return Ok(PathBuf::from("adb"));
        }

        if let Ok(android_home) = env::var("ANDROID_HOME") {
            let candidate = PathBuf::from(android_home)
                .join("platform-tools")
                .join("adb");
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        if let Ok(android_sdk_root) = env::var("ANDROID_SDK_ROOT") {
            let candidate = PathBuf::from(android_sdk_root)
                .join("platform-tools")
                .join("adb");
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(AdbError::Connect(
            "Could not locate 'adb' binary in PATH, ANDROID_HOME, or ANDROID_SDK_ROOT".into(),
        ))
    }

    /// Executes `adb start-server` to spin up the local ADB daemon process.
    pub fn start_server() -> Result<()> {
        let adb_path = Self::find_adb_binary()?;
        let status = Command::new(adb_path)
            .arg("start-server")
            .status()
            .map_err(|e| AdbError::Connect(format!("Failed to execute adb start-server: {e}")))?;

        if status.success() {
            Ok(())
        } else {
            Err(AdbError::Connect(
                "adb start-server exited with non-zero exit code".into(),
            ))
        }
    }
}
