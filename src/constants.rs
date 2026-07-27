/// ADB AUTH packet type: challenge token from device.
pub const AUTH_TYPE_TOKEN: u32 = 1;
/// ADB AUTH packet type: RSA signature response from client.
pub const AUTH_TYPE_SIGNATURE: u32 = 2;
/// ADB AUTH packet type: RSA public key transmission from client.
pub const AUTH_TYPE_RSA_PUBLIC: u32 = 3;

/// Wire command code: "AUTH" (0x48545541 in Little Endian).
pub const CMD_AUTH: u32 = 0x4854_5541;
/// Wire command code: "CNXN" (0x4E584E43 in Little Endian).
pub const CMD_CNXN: u32 = 0x4e58_4e43;
/// Wire command code: "OPEN" (0x4E45504F in Little Endian).
pub const CMD_OPEN: u32 = 0x4e45_504f;
/// Wire command code: "OKAY" (0x59414B4F in Little Endian).
pub const CMD_OKAY: u32 = 0x5941_4b4f;
/// Wire command code: "CLSE" (0x45534C43 in Little Endian).
pub const CMD_CLSE: u32 = 0x4553_4c43;
/// Wire command code: "WRTE" (0x45545257 in Little Endian).
pub const CMD_WRTE: u32 = 0x4554_5257;

/// Standard ADB connection version (A_VERSION = 0x01000000).
pub const CONNECT_VERSION: u32 = 0x0100_0000;
/// Maximum payload size negotiated during connect (1 MB).
pub const CONNECT_MAXDATA: u32 = 1_048_576;
/// Initial banner payload sent in `A_CNXN` packets.
pub const CONNECT_PAYLOAD: &[u8] = b"host::\x00";

/// Write timeout in milliseconds for socket operations.
pub const WRITE_TIMEOUT_MILLIS: u64 = 10_000;
/// Minimum port number for standard Android emulator instances.
pub const MIN_EMULATOR_PORT: u16 = 5555;
/// Maximum port number for standard Android emulator instances.
pub const MAX_EMULATOR_PORT: u16 = 5683;

/// Shell v2 protocol stream ID: standard input.
pub const SHELL_ID_STDIN: u8 = 0;
/// Shell v2 protocol stream ID: standard output.
pub const SHELL_ID_STDOUT: u8 = 1;
/// Shell v2 protocol stream ID: standard error.
pub const SHELL_ID_STDERR: u8 = 2;
/// Shell v2 protocol stream ID: exit status packet.
pub const SHELL_ID_EXIT: u8 = 3;
