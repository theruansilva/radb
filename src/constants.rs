pub const AUTH_TYPE_TOKEN: u32 = 1;
pub const AUTH_TYPE_SIGNATURE: u32 = 2;
pub const AUTH_TYPE_RSA_PUBLIC: u32 = 3;

pub const CMD_AUTH: u32 = 0x4854_5541; // "AUTH" Little Endian
pub const CMD_CNXN: u32 = 0x4e58_4e43; // "CNXN" Little Endian
pub const CMD_OPEN: u32 = 0x4e45_504f; // "OPEN" Little Endian
pub const CMD_OKAY: u32 = 0x5941_4b4f; // "OKAY" Little Endian
pub const CMD_CLSE: u32 = 0x4553_4c43; // "CLSE" Little Endian
pub const CMD_WRTE: u32 = 0x4554_5257; // "WRTE" Little Endian

pub const CONNECT_VERSION: u32 = 0x0100_0000;
pub const CONNECT_MAXDATA: u32 = 1_048_576; // 1 MB payload limit
pub const CONNECT_PAYLOAD: &[u8] = b"host::\x00";

pub const WRITE_TIMEOUT_MILLIS: u64 = 10_000;
pub const MIN_EMULATOR_PORT: u16 = 5555;
pub const MAX_EMULATOR_PORT: u16 = 5683;

pub const SHELL_ID_STDIN: u8 = 0;
pub const SHELL_ID_STDOUT: u8 = 1;
pub const SHELL_ID_STDERR: u8 = 2;
pub const SHELL_ID_EXIT: u8 = 3;
