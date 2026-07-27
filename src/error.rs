use thiserror::Error;

pub type Result<T> = std::result::Result<T, AdbError>;

#[derive(Debug, Error)]
pub enum AdbError {
    #[error("ADB Connect error: {0}")]
    Connect(String),

    #[error("ADB Auth error: {0}")]
    Auth(String),

    #[error("ADB Stream Open error for '{destination}': {reason}")]
    StreamOpen { destination: String, reason: String },

    #[error("ADB Connection Closed: {0}")]
    ConnectionClosed(String),

    #[error("ADB Timeout: {0}")]
    Timeout(String),

    #[error("ADB Protocol error: {0}")]
    Protocol(String),

    #[error("ADB Stream Closed (local_id: {0})")]
    StreamClosed(u32),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
#[error("Operation failed: {reason} (exit_code: {exit_code:?})")]
pub struct AdbOperationFailedError {
    pub reason: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallResult {
    Success,
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UninstallResult {
    Success,
    Failure { reason: String, exit_code: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootResult {
    Success,
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncResult {
    Success,
    Failure(String),
}
