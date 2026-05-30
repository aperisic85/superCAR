use crate::error::{ObdError, Result};
use crate::transport::SerialTransport;

/// Wraps SerialTransport with ELM327 AT initialisation.
pub struct Elm327 {
    transport: SerialTransport,
}

impl Elm327 {
    pub fn connect(port_path: &str, baud: u32) -> Result<Self> {
        let transport = SerialTransport::open(port_path, baud)?;
        let mut elm = Self { transport };
        elm.init()?;
        Ok(elm)
    }

    fn init(&mut self) -> Result<()> {
        self.transport.clear_buffers();

        // Warm reset — gives us a clean slate regardless of prior state
        self.at_cmd("ATZ")?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.transport.clear_buffers();

        self.at_cmd("ATE0")?;  // echo off
        self.at_cmd("ATL0")?;  // linefeed off
        self.at_cmd("ATS0")?;  // spaces off — easier to parse
        self.at_cmd("ATH0")?;  // headers off
        self.at_cmd("ATSP0")?; // auto protocol (will detect CAN on Grande Punto)

        log::info!("ELM327 initialised");
        Ok(())
    }

    /// Send a raw AT command; returns trimmed response lines.
    pub fn at_cmd(&mut self, cmd: &str) -> Result<Vec<String>> {
        let lines = self.transport.send_command(cmd)?;
        for line in &lines {
            if line.contains("ERROR") {
                return Err(ObdError::ElmError(format!("{}: {}", cmd, line)));
            }
        }
        Ok(lines)
    }

    /// Send a raw OBD hex string (e.g. "0100") and return raw hex response lines.
    pub fn send_obd(&mut self, hex_cmd: &str) -> Result<Vec<String>> {
        let lines = self.transport.send_command(hex_cmd)?;
        if lines.is_empty() {
            return Err(ObdError::NoData);
        }
        for line in &lines {
            match line.as_str() {
                "NODATA" | "NO DATA" => return Err(ObdError::NoData),
                s if s.contains("ERROR") => return Err(ObdError::ElmError(s.to_string())),
                _ => {}
            }
        }
        Ok(lines)
    }

    /// Convenience: read ELM327 firmware version string.
    pub fn elm_version(&mut self) -> Result<String> {
        let lines = self.at_cmd("ATI")?;
        Ok(lines.join(" "))
    }

    /// Describe currently detected protocol (ATDP).
    pub fn detected_protocol(&mut self) -> Result<String> {
        let lines = self.at_cmd("ATDP")?;
        Ok(lines.join(" "))
    }
}
