/// Standard EOBD/OBD-II Mode 01 PIDs relevant for Grande Punto 1.4 16v / 1.3 JTD
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pid {
    SupportedPids01_20 = 0x00,
    MonitorStatus       = 0x01,
    EngineLoad          = 0x04,
    CoolantTemp         = 0x05,
    ShortFuelTrimBank1  = 0x06,
    LongFuelTrimBank1   = 0x07,
    IntakePressure      = 0x0B,
    EngineRpm           = 0x0C,
    VehicleSpeed        = 0x0D,
    TimingAdvance       = 0x0E,
    IntakeAirTemp       = 0x0F,
    MafAirFlow          = 0x10,
    ThrottlePosition    = 0x11,
    OxygenSensors       = 0x13,
    RuntimeSinceStart   = 0x1F,
    DistanceWithMil     = 0x21,
    FuelRailPressure    = 0x23,
    CommandedEgr        = 0x2C,
    EgrError            = 0x2D,
    BarometricPressure  = 0x33,
    ControlModuleVoltage = 0x42,
    AbsoluteLoad        = 0x43,
    AmbientAirTemp      = 0x46,
    FuelType            = 0x51,
    OilTemp             = 0x5C,
}

/// Decode a Mode 01 PID response (bytes A, B, C, D as applicable).
/// Returns a human-readable string with units.
pub fn decode_pid(pid: u8, data: &[u8]) -> Option<String> {
    match pid {
        0x04 => {
            let val = data.first()?;
            Some(format!("Engine load: {:.1}%", *val as f64 * 100.0 / 255.0))
        }
        0x05 => {
            let val = data.first()?;
            Some(format!("Coolant temp: {} °C", *val as i32 - 40))
        }
        0x06 | 0x07 => {
            let val = data.first()?;
            let trim = (*val as f64 - 128.0) * 100.0 / 128.0;
            let label = if pid == 0x06 { "Short fuel trim" } else { "Long fuel trim" };
            Some(format!("{} bank1: {:.2}%", label, trim))
        }
        0x0B => {
            let val = data.first()?;
            Some(format!("Intake MAP: {} kPa", val))
        }
        0x0C => {
            if data.len() < 2 { return None; }
            let rpm = ((data[0] as u32) * 256 + data[1] as u32) / 4;
            Some(format!("Engine RPM: {}", rpm))
        }
        0x0D => {
            let val = data.first()?;
            Some(format!("Vehicle speed: {} km/h", val))
        }
        0x0E => {
            let val = data.first()?;
            let adv = *val as f64 / 2.0 - 64.0;
            Some(format!("Timing advance: {:.1}°", adv))
        }
        0x0F => {
            let val = data.first()?;
            Some(format!("Intake air temp: {} °C", *val as i32 - 40))
        }
        0x10 => {
            if data.len() < 2 { return None; }
            let maf = ((data[0] as u32) * 256 + data[1] as u32) as f64 / 100.0;
            Some(format!("MAF: {:.2} g/s", maf))
        }
        0x11 => {
            let val = data.first()?;
            Some(format!("Throttle: {:.1}%", *val as f64 * 100.0 / 255.0))
        }
        0x1F => {
            if data.len() < 2 { return None; }
            let secs = (data[0] as u32) * 256 + data[1] as u32;
            Some(format!("Run time: {}s", secs))
        }
        0x21 => {
            if data.len() < 2 { return None; }
            let km = (data[0] as u32) * 256 + data[1] as u32;
            Some(format!("Distance with MIL: {} km", km))
        }
        0x42 => {
            if data.len() < 2 { return None; }
            let v = ((data[0] as u32) * 256 + data[1] as u32) as f64 / 1000.0;
            Some(format!("Module voltage: {:.3} V", v))
        }
        0x46 => {
            let val = data.first()?;
            Some(format!("Ambient air temp: {} °C", *val as i32 - 40))
        }
        0x5C => {
            let val = data.first()?;
            Some(format!("Oil temp: {} °C", *val as i32 - 40))
        }
        _ => Some(format!("PID 0x{:02X}: {:?}", pid, data)),
    }
}
