use clap::{Parser, Subcommand};
use obd_core::{ObdSession, ObdError};

#[derive(Parser)]
#[command(
    name = "obd",
    about = "OBD-II diagnostics via ELM327 – Fiat Grande Punto 2008 / generic EOBD",
    version
)]
struct Cli {
    /// Serial port path (e.g. /dev/ttyUSB0 or COM3)
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    /// Baud rate
    #[arg(short, long, default_value_t = 38400)]
    baud: u32,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show ELM327 version and detected protocol
    Info,
    /// List supported Mode 01 PIDs
    Pids,
    /// Read a single Mode 01 PID (decimal or 0x-prefixed hex)
    Read {
        pid: String,
    },
    /// Show stored DTCs (Mode 03)
    Dtcs,
    /// Show pending DTCs (Mode 07)
    Pending,
    /// Clear stored DTCs and reset MIL (Mode 04)
    Clear,
    /// Live data: poll a set of common PIDs in a loop
    Live,
}

fn parse_pid(s: &str) -> Result<u8, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u8>().map_err(|e| e.to_string())
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    let mut session = match ObdSession::connect(&cli.port, cli.baud) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            std::process::exit(1);
        }
    };

    match cli.command {
        Command::Info => {
            match session.vehicle_info() {
                Ok(info) => {
                    println!("ELM327 : {}", info.elm_version);
                    println!("Protocol: {}", info.protocol);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Command::Pids => {
            match session.supported_pids() {
                Ok(pids) => {
                    println!("Supported PIDs (Mode 01):");
                    for pid in pids {
                        println!("  0x{:02X} ({})", pid, pid);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Command::Read { pid } => {
            let pid = match parse_pid(&pid) {
                Ok(p) => p,
                Err(e) => { eprintln!("Invalid PID: {}", e); std::process::exit(1); }
            };
            match session.read_pid(pid) {
                Ok(val) => println!("{}", val),
                Err(ObdError::UnsupportedPid(p)) => eprintln!("PID 0x{:02X} not supported by ECU", p),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Command::Dtcs => {
            match session.read_stored_dtcs() {
                Ok(dtcs) if dtcs.is_empty() => println!("No stored DTCs – all clear."),
                Ok(dtcs) => {
                    println!("{} stored DTC(s):", dtcs.len());
                    for dtc in dtcs {
                        println!("  {}", dtc);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Command::Pending => {
            match session.read_pending_dtcs() {
                Ok(dtcs) if dtcs.is_empty() => println!("No pending DTCs."),
                Ok(dtcs) => {
                    println!("{} pending DTC(s):", dtcs.len());
                    for dtc in dtcs {
                        println!("  {}", dtc);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Command::Clear => {
            print!("This will clear ALL stored DTCs and reset the MIL. Continue? [y/N] ");
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let line = stdin.lock().lines().next().unwrap_or(Ok(String::new())).unwrap_or_default();
            if line.trim().eq_ignore_ascii_case("y") {
                match session.clear_dtcs() {
                    Ok(()) => println!("DTCs cleared successfully."),
                    Err(e) => eprintln!("Error: {}", e),
                }
            } else {
                println!("Aborted.");
            }
        }

        Command::Live => {
            const LIVE_PIDS: &[(u8, &str)] = &[
                (0x0C, "RPM"),
                (0x0D, "Speed"),
                (0x05, "Coolant"),
                (0x0F, "Intake air"),
                (0x11, "Throttle"),
                (0x04, "Load"),
                (0x0E, "Timing"),
                (0x42, "Voltage"),
            ];

            println!("Live data (Ctrl-C to stop):\n");
            loop {
                for &(pid, label) in LIVE_PIDS {
                    match session.read_pid(pid) {
                        Ok(val) => println!("  {:12} {}", label, val),
                        Err(ObdError::NoData) | Err(ObdError::UnsupportedPid(_)) => {}
                        Err(e) => eprintln!("  {:12} error: {}", label, e),
                    }
                }
                println!("  {}", "─".repeat(40));
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
