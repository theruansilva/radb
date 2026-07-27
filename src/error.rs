use thiserror::Error;

/// Alias for `std::result::Result` with `AdbError`.
pub type Result<T> = std::result::Result<T, AdbError>;

/// Core error enum representing failures during ADB operations.
#[derive(Debug, Error)]
pub enum AdbError {
    /// Failure during connection establishment or handshake.
    #[error("ADB Connect error: {0}")]
    Connect(String),

    /// RSA authentication failure or keypair missing.
    #[error("ADB Auth error: {0}")]
    Auth(String),

    /// Error opening a stream channel for a specific service.
    #[error("ADB Stream Open error for '{destination}': {reason}")]
    StreamOpen {
        /// Requested destination service string.
        destination: String,
        /// Reason for failure returned by daemon or client.
        reason: String,
    },

    /// The underlying ADB connection was unexpectedly closed.
    #[error("ADB Connection Closed: {0}")]
    ConnectionClosed(String),

    /// Operation timed out.
    #[error("ADB Timeout: {0}")]
    Timeout(String),

    /// Malformed or unexpected ADB protocol packet received.
    #[error("ADB Protocol error: {0}")]
    Protocol(String),

    /// The requested stream channel was closed.
    #[error("ADB Stream Closed (local_id: {0})")]
    StreamClosed(u32),

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Represents a failed high-level ADB operation with output and optional exit code.
#[derive(Debug, Error)]
#[error("Operation failed: {reason} (exit_code: {exit_code:?})")]
pub struct AdbOperationFailedError {
    /// Reason or output string from the failed operation.
    pub reason: String,
    /// Process exit code if available.
    pub exit_code: Option<i32>,
}

/// Result of an APK installation operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallResult {
    /// Installation completed successfully.
    Success,
    /// Installation failed with detailed output error string.
    Failure(String),
}

/// Result of a package uninstallation operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UninstallResult {
    /// Uninstallation completed successfully.
    Success,
    /// Uninstallation failed with reason string and exit code.
    Failure {
        /// Output or error message from package manager.
        reason: String,
        /// Process exit code.
        exit_code: i32,
    },
}

/// Result of a root or unroot command operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootResult {
    /// ADB daemon restarted in requested root/unroot mode.
    Success,
    /// Root operation failed with output message.
    Failure(String),
}

/// Result of an ADB Sync push or pull operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncResult {
    /// File transfer completed successfully.
    Success,
    /// File transfer failed with detailed message.
    Failure(String),
}
