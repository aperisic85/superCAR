use crate::error::{ObdError, Result};
use serialport::SerialPort;
use std::io::Write;
use std::time::Duration;

const PROMPT: u8 = b'>';
const DEFAULT_TIMEOUT_MS: u64 = 2000;

pub struct SerialTransport {
    port: Box<dyn SerialPort>,
}

impl SerialTransport {
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .open()?;
        Ok(Self { port })
    }

    /// Send a raw command and read lines until the ELM327 prompt character `>`.
    pub fn send_command(&mut self, cmd: &str) -> Result<Vec<String>> {
        let full = format!("{}\r", cmd);
        log::debug!("TX: {:?}", full);
        self.port.write_all(full.as_bytes())?;
        self.port.flush()?;
        self.read_until_prompt()
    }

    fn read_until_prompt(&mut self) -> Result<Vec<String>> {
        // Read byte by byte until we see `>` so we handle varying line endings
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match self.port.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == PROMPT {
                        break;
                    }
                    buf.push(byte[0]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    return Err(ObdError::Timeout);
                }
                Err(e) => return Err(ObdError::Io(e)),
            }
        }

        let raw = String::from_utf8_lossy(&buf);
        log::debug!("RX raw: {:?}", raw);

        let lines: Vec<String> = raw
            .split(|c| c == '\r' || c == '\n')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(lines)
    }

    pub fn clear_buffers(&mut self) {
        let _ = self.port.clear(serialport::ClearBuffer::All);
    }
}
