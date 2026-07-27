use bytes::Bytes;
use std::fmt;

/// Represents an ADB wire protocol packet (24-byte header + payload).
#[derive(Clone, PartialEq, Eq)]
pub struct AdbMessage {
    /// 4-byte command identifier (e.g. `CMD_CNXN`, `CMD_OPEN`, `CMD_OKAY`).
    pub command: u32,
    /// First argument word (context dependent: local_id, version, etc.).
    pub arg0: u32,
    /// Second argument word (context dependent: remote_id, maxdata, etc.).
    pub arg1: u32,
    /// Byte length of payload.
    pub payload_length: u32,
    /// Checksum of the payload (sum of payload bytes).
    pub checksum: u32,
    /// Magic value XOR'd command (`command ^ 0xFFFFFFFF`).
    pub magic: u32,
    /// Raw packet payload data buffer.
    pub payload: Bytes,
}

impl AdbMessage {
    /// Creates a new `AdbMessage` with automatically calculated checksum and magic field.
    pub fn new(command: u32, arg0: u32, arg1: u32, payload: impl Into<Bytes>) -> Self {
        let payload = payload.into();
        let payload_length = payload.len() as u32;
        let checksum = Self::calculate_checksum(&payload);
        let magic = command ^ 0xFFFF_FFFF;

        Self {
            command,
            arg0,
            arg1,
            payload_length,
            checksum,
            magic,
            payload,
        }
    }

    /// Calculates the 32-bit checksum (sum of unsigned bytes) for a payload buffer.
    pub fn calculate_checksum(payload: &[u8]) -> u32 {
        payload.iter().map(|&byte| byte as u32).sum()
    }

    /// Returns a human-readable string representation of a command identifier.
    pub fn command_name(command: u32) -> &'static str {
        match command {
            crate::constants::CMD_AUTH => "AUTH",
            crate::constants::CMD_CNXN => "CNXN",
            crate::constants::CMD_OPEN => "OPEN",
            crate::constants::CMD_OKAY => "OKAY",
            crate::constants::CMD_CLSE => "CLSE",
            crate::constants::CMD_WRTE => "WRTE",
            _ => "UNKNOWN",
        }
    }
}

impl fmt::Debug for AdbMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdbMessage")
            .field("command", &Self::command_name(self.command))
            .field("arg0", &self.arg0)
            .field("arg1", &self.arg1)
            .field("payload_length", &self.payload_length)
            .field("checksum", &self.checksum)
            .field("magic", &format_args!("{:#X}", self.magic))
            .field("payload_len", &self.payload.len())
            .finish()
    }
}
