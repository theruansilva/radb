use crate::constants::*;
use crate::error::Result;
use crate::stream::AdbStream;
use bytes::{Buf, BytesMut};
use tokio::io::AsyncWrite;

/// Response output and exit status from a shell command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbShellResponse {
    /// Standard output (stdout) string from shell command execution.
    pub output: String,
    /// Standard error (stderr) string from shell v2 command execution.
    pub error_output: String,
    /// Exit status code returned by the shell process (0 on success).
    pub exit_code: i32,
}

impl AdbShellResponse {
    /// Returns concatenated stdout and stderr output strings.
    pub fn all_output(&self) -> String {
        format!("{}{}", self.output, self.error_output)
    }
}

/// Reads data chunks from an open shell stream and parses output using Shell v1 or Shell v2 protocol format.
pub async fn execute_shell<W: AsyncWrite + Unpin + Send + 'static>(
    stream: &mut AdbStream<W>,
    is_v2: bool,
) -> Result<AdbShellResponse> {
    if !is_v2 {
        // Shell v1: Raw stream output until EOF
        let mut all_bytes = Vec::new();
        loop {
            let chunk = stream.read_chunk().await?;
            if chunk.is_empty() {
                break;
            }
            all_bytes.extend_from_slice(&chunk);
        }
        return Ok(AdbShellResponse {
            output: String::from_utf8_lossy(&all_bytes).to_string(),
            error_output: String::new(),
            exit_code: 0,
        });
    }

    // Shell v2: Packet framing (1-byte ID + 4-byte LE len + payload)
    let mut output = String::new();
    let mut error_output = String::new();
    let mut exit_code = 0;
    let mut buf = BytesMut::new();

    let mut exited = false;
    loop {
        if exited && buf.is_empty() {
            break;
        }

        let chunk = stream.read_chunk().await?;
        if chunk.is_empty() && buf.is_empty() {
            break;
        }
        buf.extend_from_slice(&chunk);

        while buf.len() >= 5 {
            let id = buf[0];
            let payload_len = u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;

            if buf.len() < 5 + payload_len {
                break; // Wait for complete packet
            }

            buf.advance(5);
            let payload = buf.split_to(payload_len);

            match id {
                SHELL_ID_STDOUT => {
                    output.push_str(&String::from_utf8_lossy(&payload));
                }
                SHELL_ID_STDERR => {
                    error_output.push_str(&String::from_utf8_lossy(&payload));
                }
                SHELL_ID_EXIT => {
                    if !payload.is_empty() {
                        exit_code = payload[0] as i32;
                    }
                    exited = true;
                }
                _ => {}
            }
        }
    }
    Ok(AdbShellResponse {
        output,
        error_output,
        exit_code,
    })
}
