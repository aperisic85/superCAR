use crate::error::{ObdError, Result};

/// A decoded Diagnostic Trouble Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dtc {
    pub code: String,
    pub description: Option<&'static str>,
}

impl std::fmt::Display for Dtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.description {
            Some(desc) => write!(f, "{} – {}", self.code, desc),
            None => write!(f, "{} – (unknown/manufacturer-specific)", self.code),
        }
    }
}

/// Decode two raw bytes from a Mode 03/07/0A response into a DTC string.
///
/// ISO 15031-6 encoding:
///   bits 15-14: system  00=P  01=C  10=B  11=U
///   bits 13-12: digit 1 (0-3 → '0'-'3')
///   bits 11-8 : digit 2 (hex)
///   bits  7-4 : digit 3 (hex)
///   bits  3-0 : digit 4 (hex)
pub fn decode_dtc_bytes(high: u8, low: u8) -> Result<Dtc> {
    if high == 0x00 && low == 0x00 {
        return Err(ObdError::ParseError("padding byte 0x0000 is not a DTC".into()));
    }

    let system = match (high >> 6) & 0x03 {
        0 => 'P',
        1 => 'C',
        2 => 'B',
        3 => 'U',
        _ => unreachable!(),
    };
    let d1 = (high >> 4) & 0x03;
    let d2 = high & 0x0F;
    let d3 = (low >> 4) & 0x0F;
    let d4 = low & 0x0F;

    let code = format!("{}{}{:X}{:X}{:X}", system, d1, d2, d3, d4);
    let description = lookup_dtc(&code);
    Ok(Dtc { code, description })
}

/// Parse a full Mode 03 / 07 / 0A hex-line response into a Vec<Dtc>.
///
/// Each response line (after stripping mode byte 43/47/4A) contains pairs of bytes.
/// With headers off and spaces off (ATH0, ATS0), the line looks like: 430102030000
pub fn parse_dtc_response(lines: &[String]) -> Result<Vec<Dtc>> {
    let mut dtcs = Vec::new();

    for line in lines {
        // Strip leading mode-response byte (43 / 47 / 4A) if present
        let hex = if line.starts_with("43") || line.starts_with("47") || line.starts_with("4A") {
            &line[2..]
        } else {
            line.as_str()
        };

        if hex.len() % 4 != 0 {
            return Err(ObdError::ParseError(format!("odd DTC hex length: {}", hex)));
        }

        let mut i = 0;
        while i + 4 <= hex.len() {
            let high = u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| ObdError::InvalidHex(hex[i..i + 4].to_string()))?;
            let low = u8::from_str_radix(&hex[i + 2..i + 4], 16)
                .map_err(|_| ObdError::InvalidHex(hex[i..i + 4].to_string()))?;
            i += 4;

            if high == 0 && low == 0 {
                continue; // padding
            }
            dtcs.push(decode_dtc_bytes(high, low)?);
        }
    }

    Ok(dtcs)
}

/// Common EOBD/ISO powertrain codes and a few Grande Punto-specific ones.
fn lookup_dtc(code: &str) -> Option<&'static str> {
    match code {
        // ── Fuel & air metering ──────────────────────────────────────────
        "P0100" => Some("Mass Air Flow sensor – circuit malfunction"),
        "P0101" => Some("Mass Air Flow sensor – range/performance"),
        "P0102" => Some("Mass Air Flow sensor – low input"),
        "P0103" => Some("Mass Air Flow sensor – high input"),
        "P0105" => Some("MAP sensor – circuit malfunction"),
        "P0106" => Some("MAP sensor – range/performance"),
        "P0107" => Some("MAP sensor – low input"),
        "P0108" => Some("MAP sensor – high input"),
        "P0110" => Some("Intake Air Temperature sensor – circuit malfunction"),
        "P0112" => Some("Intake Air Temperature sensor – low input"),
        "P0113" => Some("Intake Air Temperature sensor – high input"),
        "P0115" => Some("Engine Coolant Temperature sensor – circuit malfunction"),
        "P0116" => Some("Engine Coolant Temperature sensor – range/performance"),
        "P0117" => Some("Engine Coolant Temperature sensor – low input"),
        "P0118" => Some("Engine Coolant Temperature sensor – high input"),
        "P0120" => Some("Throttle position sensor A – circuit malfunction"),
        "P0121" => Some("Throttle position sensor A – range/performance"),
        "P0122" => Some("Throttle position sensor A – low input"),
        "P0123" => Some("Throttle position sensor A – high input"),
        "P0130" => Some("O2 sensor B1S1 – circuit malfunction"),
        "P0131" => Some("O2 sensor B1S1 – low voltage"),
        "P0132" => Some("O2 sensor B1S1 – high voltage"),
        "P0133" => Some("O2 sensor B1S1 – slow response"),
        "P0134" => Some("O2 sensor B1S1 – no activity"),
        "P0135" => Some("O2 sensor heater B1S1 – circuit malfunction"),
        "P0136" => Some("O2 sensor B1S2 – circuit malfunction"),
        "P0137" => Some("O2 sensor B1S2 – low voltage"),
        "P0138" => Some("O2 sensor B1S2 – high voltage"),
        "P0141" => Some("O2 sensor heater B1S2 – circuit malfunction"),
        "P0170" => Some("Fuel trim B1 – malfunction"),
        "P0171" => Some("Fuel trim B1 – system too lean"),
        "P0172" => Some("Fuel trim B1 – system too rich"),
        // ── Misfire ──────────────────────────────────────────────────────
        "P0300" => Some("Random/multiple cylinder misfire"),
        "P0301" => Some("Cylinder 1 misfire"),
        "P0302" => Some("Cylinder 2 misfire"),
        "P0303" => Some("Cylinder 3 misfire"),
        "P0304" => Some("Cylinder 4 misfire"),
        // ── Catalytic converter ──────────────────────────────────────────
        "P0420" => Some("Catalyst efficiency below threshold B1"),
        // ── EGR ──────────────────────────────────────────────────────────
        "P0400" => Some("EGR flow malfunction"),
        "P0401" => Some("EGR flow insufficient"),
        "P0402" => Some("EGR flow excessive"),
        "P0403" => Some("EGR control circuit malfunction"),
        "P0404" => Some("EGR control circuit range/performance"),
        "P0405" => Some("EGR sensor A – low input"),
        "P0406" => Some("EGR sensor A – high input"),
        // ── EVAP ─────────────────────────────────────────────────────────
        "P0440" => Some("EVAP system malfunction"),
        "P0441" => Some("EVAP purge flow incorrect"),
        "P0442" => Some("EVAP small leak"),
        "P0443" => Some("EVAP purge control valve – circuit malfunction"),
        "P0446" => Some("EVAP vent control circuit malfunction"),
        "P0455" => Some("EVAP large leak"),
        "P0456" => Some("EVAP very small leak"),
        // ── Idle control ─────────────────────────────────────────────────
        "P0505" => Some("Idle control system malfunction"),
        "P0506" => Some("Idle speed too low"),
        "P0507" => Some("Idle speed too high"),
        // ── Vehicle speed / transmission ─────────────────────────────────
        "P0500" => Some("Vehicle speed sensor malfunction"),
        "P0501" => Some("Vehicle speed sensor range/performance"),
        "P0600" => Some("Serial communication link malfunction"),
        // ── Grande Punto / Fiat-specific (manufacturer P1xxx) ────────────
        "P1210" => Some("Fiat: injector circuit – low side short to battery"),
        "P1215" => Some("Fiat: fuel pressure regulator – circuit malfunction"),
        "P1351" => Some("Fiat: ignition coil primary circuit malfunction"),
        "P1352" => Some("Fiat: ignition coil secondary circuit malfunction"),
        "P1600" => Some("Fiat: ECU power supply – low voltage"),
        "P1655" => Some("Fiat: throttle actuator control – limp home mode active"),
        "P1780" => Some("Fiat: drive-by-wire throttle motor open circuit"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_p0300() {
        let dtc = decode_dtc_bytes(0x03, 0x00).unwrap();
        assert_eq!(dtc.code, "P0300");
        assert!(dtc.description.unwrap().contains("misfire"));
    }

    #[test]
    fn decode_p0171_lean() {
        let dtc = decode_dtc_bytes(0x01, 0x71).unwrap();
        assert_eq!(dtc.code, "P0171");
    }

    #[test]
    fn decode_body_code() {
        // B0101 → high=0x81, low=0x01  (bits 15-14 = 10 → B, d1=0, d2=1, d3=0, d4=1)
        let dtc = decode_dtc_bytes(0x81, 0x01).unwrap();
        assert_eq!(dtc.code, "B0101");
    }

    #[test]
    fn decode_network_code() {
        // U0100 → high=0xC1, low=0x00
        let dtc = decode_dtc_bytes(0xC1, 0x00).unwrap();
        assert_eq!(dtc.code, "U0100");
    }

    #[test]
    fn padding_bytes_skipped() {
        // 43 (mode) + 0102 (P0102) + 0000 (padding) → after stripping "43": 01020000
        let lines = vec!["4301020000".to_string()];
        let dtcs = parse_dtc_response(&lines).unwrap();
        // 0x0102 → P0102, 0x0000 → skipped
        assert_eq!(dtcs.len(), 1);
        assert_eq!(dtcs[0].code, "P0102");
    }

    #[test]
    fn parse_multiple_dtcs() {
        // 43 + 0300 + 0171  → P0300 + P0171
        let lines = vec!["4303000171".to_string()];
        let dtcs = parse_dtc_response(&lines).unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0].code, "P0300");
        assert_eq!(dtcs[1].code, "P0171");
    }

    #[test]
    fn zero_zero_is_err() {
        assert!(decode_dtc_bytes(0x00, 0x00).is_err());
    }
}
