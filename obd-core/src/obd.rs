use crate::dtc::{parse_dtc_response, Dtc};
use crate::elm327::Elm327;
use crate::error::{ObdError, Result};
use crate::pid::decode_pid;

/// High-level OBD session built on top of an Elm327 connection.
pub struct ObdSession {
    elm: Elm327,
}

#[derive(Debug, Default)]
pub struct VehicleInfo {
    pub elm_version: String,
    pub protocol: String,
}

impl ObdSession {
    pub fn connect(port: &str, baud: u32) -> Result<Self> {
        let elm = Elm327::connect(port, baud)?;
        Ok(Self { elm })
    }

    pub fn vehicle_info(&mut self) -> Result<VehicleInfo> {
        Ok(VehicleInfo {
            elm_version: self.elm.elm_version()?,
            protocol: self.elm.detected_protocol()?,
        })
    }

    // ── Mode 01 ──────────────────────────────────────────────────────────

    /// Query a single Mode 01 PID and return decoded string.
    pub fn read_pid(&mut self, pid: u8) -> Result<String> {
        let cmd = format!("01{:02X}", pid);
        let lines = self.elm.send_obd(&cmd)?;

        // Expected line: 41XX<data bytes>  (mode response 01+0x40=0x41)
        let line = lines.first().ok_or(ObdError::NoData)?;
        let hex = strip_mode_byte(line, 0x41)?;
        let data = parse_hex_bytes(&hex)?;

        decode_pid(pid, &data).ok_or(ObdError::UnsupportedPid(pid))
    }

    /// Check which PIDs are supported (Mode 01 PID 00 bitmap).
    pub fn supported_pids(&mut self) -> Result<Vec<u8>> {
        let lines = self.elm.send_obd("0100")?;
        let line = lines.first().ok_or(ObdError::NoData)?;
        let hex = strip_mode_byte(line, 0x41)?;
        // skip PID echo byte (first byte after mode)
        let hex = if hex.len() >= 2 { &hex[2..] } else { &hex };
        let bytes = parse_hex_bytes(hex)?;

        let mut pids = Vec::new();
        for (byte_idx, &b) in bytes.iter().enumerate() {
            for bit in 0..8u8 {
                if b & (0x80 >> bit) != 0 {
                    pids.push(byte_idx as u8 * 8 + bit + 1);
                }
            }
        }
        Ok(pids)
    }

    // ── Mode 03 – stored DTCs ─────────────────────────────────────────────

    pub fn read_stored_dtcs(&mut self) -> Result<Vec<Dtc>> {
        match self.elm.send_obd("03") {
            Ok(lines) => parse_dtc_response(&lines),
            Err(ObdError::NoData) => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    // ── Mode 04 – clear DTCs ──────────────────────────────────────────────

    /// Clear all stored DTCs and reset readiness monitors.
    /// Returns Ok(()) when the ECU acknowledged with 0x44.
    pub fn clear_dtcs(&mut self) -> Result<()> {
        let lines = self.elm.send_obd("04")?;
        let line = lines.first().ok_or(ObdError::NoData)?;
        if line.contains("44") || line.is_empty() {
            Ok(())
        } else {
            Err(ObdError::ParseError(format!("Unexpected Mode 04 response: {}", line)))
        }
    }

    // ── Mode 07 – pending DTCs ────────────────────────────────────────────

    pub fn read_pending_dtcs(&mut self) -> Result<Vec<Dtc>> {
        match self.elm.send_obd("07") {
            Ok(lines) => parse_dtc_response(&lines),
            Err(ObdError::NoData) => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}

// ── Hex parsing helpers ───────────────────────────────────────────────────

fn strip_mode_byte(line: &str, expected_mode: u8) -> Result<String> {
    let mode_hex = format!("{:02X}", expected_mode);
    if line.starts_with(&mode_hex) {
        Ok(line[2..].to_string())
    } else {
        // Some adapters include the full frame; search for the mode byte
        if let Some(pos) = line.find(&mode_hex) {
            Ok(line[pos + 2..].to_string())
        } else {
            Err(ObdError::ParseError(format!(
                "Expected mode byte {:02X} in '{}'",
                expected_mode, line
            )))
        }
    }
}

fn parse_hex_bytes(hex: &str) -> Result<Vec<u8>> {
    let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err(ObdError::InvalidHex(hex.to_string()));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|_| ObdError::InvalidHex(clean[i..i + 2].to_string()))
        })
        .collect()
}
