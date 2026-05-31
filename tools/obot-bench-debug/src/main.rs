use std::{
    env, fs, io,
    os::{
        fd::AsRawFd,
        raw::{c_int, c_ulong, c_void},
    },
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use obot_core::{
    ControlMode, MotorCommand,
    power::{BusVoltageCalibration, OutputGate},
    text_api::{ApiCatalog, ApiDispatchError, ApiEntry, ApiValue},
};
use obot_protocol::{
    BENCHMARK_PACKET_LEN, BUS_VOLTAGE_PACKET_LEN, BenchmarkPacket, BusVoltagePacket, CommandPacket,
    DRIVER_REPORT_PACKET_LEN, DriverCommand, DriverCommandPacket, DriverReportPacket,
    OUTPUT_SAFETY_PACKET_LEN, OutputSafetyPacket, STATUS_PACKET_LEN, StatusPacket,
    TEXT_API_RESPONSE_PACKET_LEN, TextApiRequestPacket, TextApiResponsePacket,
    TextApiResponseStatus, usb_control,
};

const DEFAULT_NAME: &str = "rust_debug";
const DEFAULT_DEVICE: &str = "STM32G474RE";
const DEFAULT_ADDRESS: u32 = 0x2000_0000;
const DEFAULT_SPEED_KHZ: u32 = 4_000;
const DEFAULT_ELF_PATH: &str = "target/thumbv7em-none-eabihf/release/obot-g474";
const CYCLES_PER_100_US: u64 = 17_000;
const FAST_LOOPS_PER_MAIN_LOOP: u64 = 5;
const BENCHMARK_PACKET_SYMBOL: &str = "OBOT_BENCHMARK_PACKET";
const BUS_VOLTAGE_PACKET_SYMBOL: &str = "OBOT_BUS_VOLTAGE_PACKET";
const COMMAND_PACKET_SYMBOL: &str = "OBOT_COMMAND_PACKET";
const COMMAND_SEQUENCE_SYMBOL: &str = "OBOT_COMMAND_PACKET_SEQUENCE";
const DRIVER_COMMAND_PACKET_SYMBOL: &str = "OBOT_DRIVER_COMMAND_PACKET";
const DRIVER_COMMAND_SEQUENCE_SYMBOL: &str = "OBOT_DRIVER_COMMAND_PACKET_SEQUENCE";
const DRIVER_REPORT_PACKET_SYMBOL: &str = "OBOT_DRIVER_REPORT_PACKET";
const OUTPUT_SAFETY_PACKET_SYMBOL: &str = "OBOT_OUTPUT_SAFETY_PACKET";
const STATUS_PACKET_SYMBOL: &str = "OBOT_STATUS_PACKET";
const TEXT_API_REQUEST_PACKET_SYMBOL: &str = "OBOT_TEXT_API_REQUEST_PACKET";
const TEXT_API_REQUEST_SEQUENCE_SYMBOL: &str = "OBOT_TEXT_API_REQUEST_PACKET_SEQUENCE";
const TEXT_API_RESPONSE_PACKET_SYMBOL: &str = "OBOT_TEXT_API_RESPONSE_PACKET";
const DEFAULT_USB_TIMEOUT_MS: u32 = 1_000;
const USB_REALTIME_INTERFACE: u32 = 0;
const USB_REALTIME_OUT_ENDPOINT: u32 = 0x02;
const USB_REALTIME_IN_ENDPOINT: u32 = 0x82;
const USB_TEXT_OUT_ENDPOINT: u32 = 0x01;
const USB_TEXT_IN_ENDPOINT: u32 = 0x81;
const USBDEVFS_BULK: c_ulong = 0xC018_5502;
const USBDEVFS_CLAIMINTERFACE: c_ulong = 0x8004_550F;
const USBDEVFS_RELEASEINTERFACE: c_ulong = 0x8004_5510;
const HEAP_ALLOCATOR_SYMBOLS: &[&str] = &[
    "__rust_alloc",
    "__rust_dealloc",
    "__rust_realloc",
    "__rust_alloc_zeroed",
    "__rust_alloc_error_handler",
    "__rg_oom",
    "__rdl_alloc",
    "__rdl_dealloc",
    "__rdl_realloc",
    "__rdl_alloc_zeroed",
];

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{}", usage());
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let (command, rest) = args
        .split_first()
        .ok_or_else(|| "missing command".to_string())?;

    match command.as_str() {
        "decode-hex" => decode_hex_command(rest),
        "decode-file" => decode_file_command(rest),
        "decode-detail-hex" => decode_detail_hex_command(rest),
        "decode-detail-file" => decode_detail_file_command(rest),
        "decode-status-hex" => decode_status_hex_command(rest),
        "decode-driver-hex" => decode_driver_hex_command(rest),
        "decode-output-safety-hex" => decode_output_safety_hex_command(rest),
        "decode-bus-voltage-hex" => decode_bus_voltage_hex_command(rest),
        "decode-text-api-response-hex" => decode_text_api_response_hex_command(rest),
        "read-jlink" => read_jlink_command(rest),
        "read-jlink-detail" => read_jlink_detail_command(rest),
        "run-stats-jlink" => run_stats_jlink_command(rest),
        "run-stats-usb" => run_stats_usb_command(rest),
        "verify-no-heap" => verify_no_heap_command(rest),
        "read-status-jlink" => read_status_jlink_command(rest),
        "read-driver-jlink" => read_driver_jlink_command(rest),
        "read-output-safety-jlink" => read_output_safety_jlink_command(rest),
        "read-bus-voltage-jlink" => read_bus_voltage_jlink_command(rest),
        "snapshot-jlink" => snapshot_jlink_command(rest),
        "api-snapshot-jlink" => api_snapshot_jlink_command(rest),
        "read-text-api-response-jlink" => read_text_api_response_jlink_command(rest),
        "read-text-api-usb" => read_text_api_usb_command(rest),
        "write-text-api-request-jlink" => write_text_api_request_jlink_command(rest),
        "write-command-jlink" => write_command_jlink_command(rest),
        "write-command-usb" => write_command_usb_command(rest),
        "write-driver-command-jlink" => write_driver_command_jlink_command(rest),
        "jlink-script" => jlink_script_command(rest),
        "--help" | "-h" | "help" => Ok(usage()),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn decode_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_packet_csv(&bytes)
}

fn decode_file_command(args: &[String]) -> Result<String, String> {
    let path = args
        .first()
        .ok_or_else(|| "decode-file requires a path".to_string())?;
    if args.len() > 1 {
        return Err("decode-file accepts exactly one path".to_string());
    }

    let bytes = fs::read(path).map_err(|error| format!("failed to read `{path}`: {error}"))?;
    decode_packet_csv(&bytes)
}

fn decode_detail_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-detail-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_packet_detail_csv(&bytes)
}

fn decode_detail_file_command(args: &[String]) -> Result<String, String> {
    let path = args
        .first()
        .ok_or_else(|| "decode-detail-file requires a path".to_string())?;
    if args.len() > 1 {
        return Err("decode-detail-file accepts exactly one path".to_string());
    }

    let bytes = fs::read(path).map_err(|error| format!("failed to read `{path}`: {error}"))?;
    decode_packet_detail_csv(&bytes)
}

fn decode_status_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-status-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_status_csv(&bytes)
}

fn decode_driver_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-driver-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_driver_csv(&bytes)
}

fn decode_output_safety_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-output-safety-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_output_safety_csv(&bytes)
}

fn decode_bus_voltage_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-bus-voltage-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_bus_voltage_csv(&bytes)
}

fn decode_text_api_response_hex_command(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("decode-text-api-response-hex requires packet bytes".to_string());
    }

    let bytes = parse_hex_bytes(&args.join(" "))?;
    decode_text_api_response_csv(&bytes)
}

fn read_jlink_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    let bytes = read_jlink_bytes(&options, BENCHMARK_PACKET_LEN)?;
    decode_packet_csv(&bytes)
}

fn read_jlink_detail_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    let bytes = read_jlink_bytes(&options, BENCHMARK_PACKET_LEN)?;
    decode_packet_detail_csv(&bytes)
}

fn run_stats_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(BENCHMARK_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, BENCHMARK_PACKET_LEN)?;
    decode_packet_csv(&bytes)
}

fn verify_no_heap_command(args: &[String]) -> Result<String, String> {
    let options = ElfOnlyOptions::parse(args)?;
    let symbols = heap_allocator_symbols_in_elf(&options.elf_path)?;
    if !symbols.is_empty() {
        return Err(format!(
            "firmware ELF `{}` references heap allocator symbols: {}",
            options.elf_path.display(),
            symbols.join(", ")
        ));
    }

    Ok(format!(
        "no heap allocator symbols found in {}
",
        options.elf_path.display()
    ))
}

fn run_stats_usb_command(args: &[String]) -> Result<String, String> {
    let options = UsbRunStatsOptions::parse(args)?;
    let stats = read_usb_run_stats(&options)?;
    Ok(format_usb_run_stats_csv(DEFAULT_NAME, stats))
}

fn read_text_api_usb_command(args: &[String]) -> Result<String, String> {
    let options = TextApiUsbOptions::parse(args)?;
    Ok(format!("{}\n", read_text_api_usb(&options)?))
}

fn read_status_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(STATUS_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, STATUS_PACKET_LEN)?;
    decode_status_csv(&bytes)
}

fn read_driver_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(DRIVER_REPORT_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, DRIVER_REPORT_PACKET_LEN)?;
    decode_driver_csv(&bytes)
}

fn read_output_safety_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(OUTPUT_SAFETY_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, OUTPUT_SAFETY_PACKET_LEN)?;
    decode_output_safety_csv(&bytes)
}

fn read_bus_voltage_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(BUS_VOLTAGE_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, BUS_VOLTAGE_PACKET_LEN)?;
    decode_bus_voltage_csv(&bytes)
}

fn read_text_api_response_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(TEXT_API_RESPONSE_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, TEXT_API_RESPONSE_PACKET_LEN)?;
    decode_text_api_response_csv(&bytes)
}

fn snapshot_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    if options.address.is_some() {
        return Err(
            "snapshot-jlink reads multiple symbols; use --elf instead of --address".to_string(),
        );
    }

    Ok(format_snapshot_csv(
        DEFAULT_NAME,
        read_debug_snapshot(&options)?,
    ))
}

fn api_snapshot_jlink_command(args: &[String]) -> Result<String, String> {
    let options = ApiSnapshotOptions::parse(args)?;
    let snapshot = read_debug_snapshot(&options.symbols)?;
    let entries = snapshot_api_entries(snapshot);
    let catalog = ApiCatalog::new(&entries);
    let mut response = [0; 96];
    let response = catalog
        .dispatch(&options.request, &mut response)
        .map_err(format_api_dispatch_error)?;

    Ok(format!("{response}\n"))
}

fn read_debug_snapshot(options: &SymbolReadOptions) -> Result<DebugSnapshot, String> {
    let benchmark = decode_packet(&read_symbol_bytes(
        options,
        BENCHMARK_PACKET_SYMBOL,
        BENCHMARK_PACKET_LEN,
    )?)?;
    let status = decode_status_packet(&read_symbol_bytes(
        options,
        STATUS_PACKET_SYMBOL,
        STATUS_PACKET_LEN,
    )?)?;
    let driver = decode_driver_report_packet(&read_symbol_bytes(
        options,
        DRIVER_REPORT_PACKET_SYMBOL,
        DRIVER_REPORT_PACKET_LEN,
    )?)?;
    let output_safety = decode_output_safety_packet(&read_symbol_bytes(
        options,
        OUTPUT_SAFETY_PACKET_SYMBOL,
        OUTPUT_SAFETY_PACKET_LEN,
    )?)?;
    let bus_voltage = decode_bus_voltage_packet(&read_symbol_bytes(
        options,
        BUS_VOLTAGE_PACKET_SYMBOL,
        BUS_VOLTAGE_PACKET_LEN,
    )?)?;

    Ok(DebugSnapshot {
        benchmark,
        status,
        driver,
        output_safety,
        bus_voltage,
    })
}

fn read_symbol_bytes(
    options: &SymbolReadOptions,
    symbol: &str,
    len: usize,
) -> Result<Vec<u8>, String> {
    let jlink = options.resolve(symbol)?;
    read_jlink_bytes(&jlink, len)
}

fn read_jlink_bytes(options: &JlinkOptions, len: usize) -> Result<Vec<u8>, String> {
    let script = jlink_read_script(options, len);
    let script_path = write_temp_script(&script)?;
    let output = Command::new("JLinkExe")
        .arg("-CommanderScript")
        .arg(&script_path)
        .output()
        .map_err(|error| format!("failed to run JLinkExe: {error}"));
    let _ = fs::remove_file(&script_path);
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "JLinkExe failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }

    parse_jlink_mem8_output(&stdout, len)
}

fn jlink_script_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    Ok(jlink_script(&options))
}

fn write_command_jlink_command(args: &[String]) -> Result<String, String> {
    let options = CommandWriteOptions::parse(args)?.resolve()?;
    let packet = CommandPacket {
        sequence: options.sequence,
        command: MotorCommand {
            mode: options.mode,
            torque_nm: options.torque_nm,
            velocity_rad_s: options.velocity_rad_s,
            position_rad: options.position_rad,
        },
    };
    let encoded = packet.encode();
    let script = jlink_write_command_script(&options, &encoded);
    let script_path = write_temp_script(&script)?;
    let output = Command::new("JLinkExe")
        .arg("-CommanderScript")
        .arg(&script_path)
        .output()
        .map_err(|error| format!("failed to run JLinkExe: {error}"));
    let _ = fs::remove_file(&script_path);
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "JLinkExe failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }

    Ok(format!(
        "wrote command sequence {} mode {:?}\n",
        options.sequence, options.mode
    ))
}

fn write_command_usb_command(args: &[String]) -> Result<String, String> {
    let options = RealtimeUsbOptions::parse(args)?;
    let packet = CommandPacket {
        sequence: options.sequence,
        command: MotorCommand {
            mode: options.mode,
            torque_nm: options.torque_nm,
            velocity_rad_s: options.velocity_rad_s,
            position_rad: options.position_rad,
        },
    };
    let status = transact_realtime_usb(&options, packet)?;
    Ok(format_status_csv(DEFAULT_NAME, status))
}

fn write_driver_command_jlink_command(args: &[String]) -> Result<String, String> {
    let options = DriverCommandWriteOptions::parse(args)?.resolve()?;
    let packet = DriverCommandPacket {
        sequence: options.sequence,
        command: options.command,
    };
    let encoded = packet.encode();
    let script = jlink_write_raw_packet_script(
        &options.jlink,
        options.packet_address,
        options.sequence_address,
        options.sequence,
        &encoded,
    );
    let script_path = write_temp_script(&script)?;
    let output = Command::new("JLinkExe")
        .arg("-CommanderScript")
        .arg(&script_path)
        .output()
        .map_err(|error| format!("failed to run JLinkExe: {error}"));
    let _ = fs::remove_file(&script_path);
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "JLinkExe failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }

    Ok(format!(
        "wrote driver command sequence {} command {:?}\n",
        options.sequence, options.command
    ))
}

fn write_text_api_request_jlink_command(args: &[String]) -> Result<String, String> {
    let options = TextApiRequestWriteOptions::parse(args)?.resolve()?;
    let packet = TextApiRequestPacket::new(options.sequence, &options.request)
        .map_err(|error| format!("invalid text API request: {error:?}"))?;
    let encoded = packet.encode();
    let script = jlink_write_raw_packet_script(
        &options.jlink,
        options.packet_address,
        options.sequence_address,
        options.sequence,
        &encoded,
    );
    let script_path = write_temp_script(&script)?;
    let output = Command::new("JLinkExe")
        .arg("-CommanderScript")
        .arg(&script_path)
        .output()
        .map_err(|error| format!("failed to run JLinkExe: {error}"));
    let _ = fs::remove_file(&script_path);
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "JLinkExe failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }

    Ok(format!(
        "wrote text API request sequence {} request {}\n",
        options.sequence, options.request
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct JlinkOptions {
    address: u32,
    speed_khz: u32,
    device: &'static str,
}

impl JlinkOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            address: DEFAULT_ADDRESS,
            speed_khz: DEFAULT_SPEED_KHZ,
            device: DEFAULT_DEVICE,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--address" => {
                    index += 1;
                    options.address = parse_u32_arg(args.get(index), "--address")?;
                }
                "--speed" => {
                    index += 1;
                    options.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SymbolReadOptions {
    jlink: JlinkOptions,
    address: Option<u32>,
    elf_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ElfOnlyOptions {
    elf_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApiSnapshotOptions {
    symbols: SymbolReadOptions,
    request: String,
}

impl ElfOnlyOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    options.elf_path = PathBuf::from(path);
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }
}

impl ApiSnapshotOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut symbols = SymbolReadOptions {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            address: None,
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
        };
        let mut request = None;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    symbols.elf_path = PathBuf::from(path);
                }
                "--speed" => {
                    index += 1;
                    symbols.jlink.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                "--address" => {
                    return Err(
                        "api-snapshot-jlink reads multiple symbols; use --elf instead of --address"
                            .to_string(),
                    );
                }
                value => {
                    if request.is_some() {
                        return Err(format!("unexpected extra request argument `{value}`"));
                    }
                    request = Some(value.to_string());
                }
            }
            index += 1;
        }

        Ok(Self {
            symbols,
            request: request
                .ok_or_else(|| "api-snapshot-jlink requires a text API request".to_string())?,
        })
    }
}

impl SymbolReadOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            address: None,
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--address" => {
                    index += 1;
                    options.address = Some(parse_u32_arg(args.get(index), "--address")?);
                }
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    options.elf_path = PathBuf::from(path);
                }
                "--speed" => {
                    index += 1;
                    options.jlink.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }

    fn resolve(&self, symbol: &str) -> Result<JlinkOptions, String> {
        let address = match self.address {
            Some(address) => address,
            None => resolve_symbol_address(&self.elf_path, symbol)?,
        };

        Ok(JlinkOptions {
            address,
            speed_khz: self.jlink.speed_khz,
            device: self.jlink.device,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RealtimeUsbOptions {
    device_path: Option<PathBuf>,
    sequence: u8,
    mode: ControlMode,
    torque_nm: f32,
    velocity_rad_s: f32,
    position_rad: f32,
    timeout_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct TextApiUsbOptions {
    device_path: Option<PathBuf>,
    request: String,
    timeout_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct UsbRunStatsOptions {
    device_path: Option<PathBuf>,
    samples: usize,
    timeout_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UsbRunStats {
    max_fast_loop_cycles: u32,
    max_fast_loop_period: u32,
    max_main_loop_cycles: u32,
    max_main_loop_period: u32,
    mean_fast_loop_cycles_milli: u64,
    mean_fast_loop_period_milli: u64,
    mean_main_loop_cycles_milli: u64,
    mean_main_loop_period_milli: u64,
}

#[repr(C)]
struct UsbdevfsBulkTransfer {
    ep: u32,
    len: u32,
    timeout: u32,
    data: *mut c_void,
}

struct UsbInterfaceClaim<'file> {
    file: &'file fs::File,
    interface: u32,
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

#[derive(Clone, Debug, PartialEq)]
struct CommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: Option<u32>,
    sequence_address: Option<u32>,
    elf_path: PathBuf,
    sequence: u8,
    mode: ControlMode,
    torque_nm: f32,
    velocity_rad_s: f32,
    position_rad: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedCommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    mode: ControlMode,
    torque_nm: f32,
    velocity_rad_s: f32,
    position_rad: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct DriverCommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: Option<u32>,
    sequence_address: Option<u32>,
    elf_path: PathBuf,
    sequence: u8,
    command: DriverCommand,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedDriverCommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    command: DriverCommand,
}

#[derive(Clone, Debug, PartialEq)]
struct TextApiRequestWriteOptions {
    jlink: JlinkOptions,
    packet_address: Option<u32>,
    sequence_address: Option<u32>,
    elf_path: PathBuf,
    sequence: u8,
    request: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedTextApiRequestWriteOptions {
    jlink: JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    request: String,
}

impl TextApiUsbOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            device_path: None,
            request: String::new(),
            timeout_ms: DEFAULT_USB_TIMEOUT_MS,
        };
        let mut request = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dev" | "--device-path" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--dev requires a value".to_string())?;
                    options.device_path = Some(PathBuf::from(path));
                }
                "--timeout-ms" => {
                    index += 1;
                    options.timeout_ms = parse_u32_arg(args.get(index), "--timeout-ms")?;
                }
                value => {
                    if request.is_some() {
                        return Err(format!(
                            "unexpected extra text API request argument `{value}`"
                        ));
                    }
                    request = Some(value.to_string());
                }
            }
            index += 1;
        }

        options.request =
            request.ok_or_else(|| "read-text-api-usb requires a request".to_string())?;
        Ok(options)
    }

    fn resolved_device_path(&self) -> Result<PathBuf, String> {
        resolve_usb_device_path(self.device_path.as_ref())
    }
}

impl UsbRunStatsOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            device_path: None,
            samples: 100,
            timeout_ms: DEFAULT_USB_TIMEOUT_MS,
        };
        let mut positional_samples = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dev" | "--device-path" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--dev requires a value".to_string())?;
                    options.device_path = Some(PathBuf::from(path));
                }
                "--samples" => {
                    index += 1;
                    let samples = parse_u32_arg(args.get(index), "--samples")?;
                    options.samples = samples
                        .try_into()
                        .map_err(|_| "--samples does not fit in usize".to_string())?;
                }
                "--timeout-ms" => {
                    index += 1;
                    options.timeout_ms = parse_u32_arg(args.get(index), "--timeout-ms")?;
                }
                value => {
                    if positional_samples.is_some() {
                        return Err(format!("unexpected extra sample-count argument `{value}`"));
                    }
                    positional_samples = Some(
                        parse_u32(value)
                            .ok_or_else(|| format!("invalid sample-count argument `{value}`"))?,
                    );
                }
            }
            index += 1;
        }
        if let Some(samples) = positional_samples {
            options.samples = samples
                .try_into()
                .map_err(|_| "sample count does not fit in usize".to_string())?;
        }
        if options.samples == 0 {
            return Err("run-stats-usb requires at least one sample".to_string());
        }
        Ok(options)
    }

    fn resolved_device_path(&self) -> Result<PathBuf, String> {
        resolve_usb_device_path(self.device_path.as_ref())
    }
}

impl RealtimeUsbOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            device_path: None,
            sequence: 1,
            mode: ControlMode::Disabled,
            torque_nm: 0.0,
            velocity_rad_s: 0.0,
            position_rad: 0.0,
            timeout_ms: DEFAULT_USB_TIMEOUT_MS,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--dev" | "--device-path" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--dev requires a value".to_string())?;
                    options.device_path = Some(PathBuf::from(path));
                }
                "--sequence" => {
                    index += 1;
                    let sequence = parse_u32_arg(args.get(index), "--sequence")?;
                    options.sequence = sequence
                        .try_into()
                        .map_err(|_| "--sequence must fit in u8".to_string())?;
                }
                "--mode" => {
                    index += 1;
                    let mode = args
                        .get(index)
                        .ok_or_else(|| "--mode requires a value".to_string())?;
                    options.mode = parse_control_mode(mode)?;
                }
                "--torque" => {
                    index += 1;
                    options.torque_nm = parse_f32_arg(args.get(index), "--torque")?;
                }
                "--velocity" => {
                    index += 1;
                    options.velocity_rad_s = parse_f32_arg(args.get(index), "--velocity")?;
                }
                "--position" => {
                    index += 1;
                    options.position_rad = parse_f32_arg(args.get(index), "--position")?;
                }
                "--timeout-ms" => {
                    index += 1;
                    options.timeout_ms = parse_u32_arg(args.get(index), "--timeout-ms")?;
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }

    fn resolved_device_path(&self) -> Result<PathBuf, String> {
        resolve_usb_device_path(self.device_path.as_ref())
    }
}

fn resolve_usb_device_path(device_path: Option<&PathBuf>) -> Result<PathBuf, String> {
    match device_path {
        Some(path) => Ok(path.clone()),
        None => discover_obot_usb_device_path(),
    }
}

fn discover_obot_usb_device_path() -> Result<PathBuf, String> {
    discover_usb_device_path(
        Path::new("/sys/bus/usb/devices"),
        Path::new("/dev/bus/usb"),
        usb_control::VENDOR_ID,
        usb_control::PRODUCT_ID,
    )
}

fn discover_usb_device_path(
    sysfs_root: &Path,
    usbfs_root: &Path,
    vendor_id: u16,
    product_id: u16,
) -> Result<PathBuf, String> {
    let entries = fs::read_dir(sysfs_root)
        .map_err(|error| format!("failed to read `{}`: {error}", sysfs_root.display()))?;
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("failed to inspect `{}`: {error}", sysfs_root.display()))?;
        let path = entry.path();
        if !usb_identity_matches(&path, vendor_id, product_id)? {
            continue;
        }
        let bus = read_sysfs_u16(&path.join("busnum"))?;
        let device = read_sysfs_u16(&path.join("devnum"))?;
        matches.push(format_usbfs_path(usbfs_root, bus, device));
    }

    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "no OBOT USB device {:04x}:{:04x} found; pass --dev /dev/bus/usb/<bus>/<dev>",
            vendor_id, product_id
        )),
        _ => Err(format!(
            "multiple OBOT USB devices {:04x}:{:04x} found: {}; pass --dev to choose one",
            vendor_id,
            product_id,
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn usb_identity_matches(path: &Path, vendor_id: u16, product_id: u16) -> Result<bool, String> {
    let id_vendor = path.join("idVendor");
    let id_product = path.join("idProduct");
    if !id_vendor.exists() || !id_product.exists() {
        return Ok(false);
    }

    Ok(read_sysfs_hex_u16(&id_vendor)? == vendor_id
        && read_sysfs_hex_u16(&id_product)? == product_id)
}

fn read_sysfs_hex_u16(path: &Path) -> Result<u16, String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    u16::from_str_radix(value.trim(), 16).map_err(|_| {
        format!(
            "invalid hex value `{}` in `{}`",
            value.trim(),
            path.display()
        )
    })
}

fn read_sysfs_u16(path: &Path) -> Result<u16, String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("failed to read `{}`: {error}", path.display()))?;
    value.trim().parse().map_err(|_| {
        format!(
            "invalid decimal value `{}` in `{}`",
            value.trim(),
            path.display()
        )
    })
}

fn format_usbfs_path(root: &Path, bus: u16, device: u16) -> PathBuf {
    root.join(format!("{bus:03}")).join(format!("{device:03}"))
}

impl Drop for UsbInterfaceClaim<'_> {
    fn drop(&mut self) {
        let mut interface = self.interface;
        let _ = unsafe {
            ioctl(
                self.file.as_raw_fd(),
                USBDEVFS_RELEASEINTERFACE,
                &mut interface,
            )
        };
    }
}

fn read_text_api_usb(options: &TextApiUsbOptions) -> Result<String, String> {
    let device_path = options.resolved_device_path()?;
    let file = open_usb_device(&device_path)?;
    let _claim = claim_usb_interface(&file, USB_REALTIME_INTERFACE)?;
    text_api_usb_request_on_file(&file, &options.request, options.timeout_ms)
}

fn read_usb_run_stats(options: &UsbRunStatsOptions) -> Result<UsbRunStats, String> {
    let device_path = options.resolved_device_path()?;
    let file = open_usb_device(&device_path)?;
    let _claim = claim_usb_interface(&file, USB_REALTIME_INTERFACE)?;

    let mut stats = UsbRunStatsAccumulator::default();
    for _ in 0..options.samples {
        let fast_cycles = text_api_usb_u32_on_file(&file, "t_exec_fastloop", options.timeout_ms)?;
        let fast_period = text_api_usb_u32_on_file(&file, "t_period_fastloop", options.timeout_ms)?;
        let main_cycles = text_api_usb_u32_on_file(&file, "t_exec_mainloop", options.timeout_ms)?;
        let main_period = text_api_usb_u32_on_file(&file, "t_period_mainloop", options.timeout_ms)?;
        stats.push(fast_cycles, fast_period, main_cycles, main_period);
    }

    Ok(stats.finish())
}

fn open_usb_device(path: &Path) -> Result<fs::File, String> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to open `{}`: {error}", path.display()))
}

fn text_api_usb_u32_on_file(
    file: &fs::File,
    request: &str,
    timeout_ms: u32,
) -> Result<u32, String> {
    let response = text_api_usb_request_on_file(file, request, timeout_ms)?;
    response
        .trim()
        .parse()
        .map_err(|_| format!("text API `{request}` returned non-u32 response `{response}`"))
}

fn text_api_usb_request_on_file(
    file: &fs::File,
    request: &str,
    timeout_ms: u32,
) -> Result<String, String> {
    if request.len() > usb_control::BULK_MAX_PACKET_SIZE as usize {
        return Err(format!(
            "text API request is {} bytes; maximum is {}",
            request.len(),
            usb_control::BULK_MAX_PACKET_SIZE
        ));
    }

    let mut request_bytes = request.as_bytes().to_vec();
    let written = usbfs_bulk_transfer(file, USB_TEXT_OUT_ENDPOINT, &mut request_bytes, timeout_ms)?;
    if written != request_bytes.len() {
        return Err(format!(
            "short text API write: wrote {written} of {} bytes",
            request_bytes.len()
        ));
    }

    let mut response = [0; usb_control::BULK_MAX_PACKET_SIZE as usize];
    let read = usbfs_bulk_transfer(file, USB_TEXT_IN_ENDPOINT, &mut response, timeout_ms)?;
    std::str::from_utf8(&response[..read])
        .map(|response| response.to_string())
        .map_err(|error| format!("text API response was not UTF-8: {error}"))
}

#[derive(Default)]
struct UsbRunStatsAccumulator {
    samples: u64,
    max_fast_loop_cycles: u32,
    max_fast_loop_period: u32,
    max_main_loop_cycles: u32,
    max_main_loop_period: u32,
    sum_fast_loop_cycles: u64,
    sum_fast_loop_period: u64,
    sum_main_loop_cycles: u64,
    sum_main_loop_period: u64,
}

impl UsbRunStatsAccumulator {
    fn push(&mut self, fast_cycles: u32, fast_period: u32, main_cycles: u32, main_period: u32) {
        self.samples += 1;
        self.max_fast_loop_cycles = self.max_fast_loop_cycles.max(fast_cycles);
        self.max_fast_loop_period = self.max_fast_loop_period.max(fast_period);
        self.max_main_loop_cycles = self.max_main_loop_cycles.max(main_cycles);
        self.max_main_loop_period = self.max_main_loop_period.max(main_period);
        self.sum_fast_loop_cycles += fast_cycles as u64;
        self.sum_fast_loop_period += fast_period as u64;
        self.sum_main_loop_cycles += main_cycles as u64;
        self.sum_main_loop_period += main_period as u64;
    }

    fn finish(self) -> UsbRunStats {
        UsbRunStats {
            max_fast_loop_cycles: self.max_fast_loop_cycles,
            max_fast_loop_period: self.max_fast_loop_period,
            max_main_loop_cycles: self.max_main_loop_cycles,
            max_main_loop_period: self.max_main_loop_period,
            mean_fast_loop_cycles_milli: mean_milli(self.sum_fast_loop_cycles, self.samples),
            mean_fast_loop_period_milli: mean_milli(self.sum_fast_loop_period, self.samples),
            mean_main_loop_cycles_milli: mean_milli(self.sum_main_loop_cycles, self.samples),
            mean_main_loop_period_milli: mean_milli(self.sum_main_loop_period, self.samples),
        }
    }
}

fn mean_milli(sum: u64, samples: u64) -> u64 {
    (sum * 1_000 + samples / 2) / samples
}

fn transact_realtime_usb(
    options: &RealtimeUsbOptions,
    packet: CommandPacket,
) -> Result<StatusPacket, String> {
    let device_path = options.resolved_device_path()?;
    let file = open_usb_device(&device_path)?;
    let _claim = claim_usb_interface(&file, USB_REALTIME_INTERFACE)?;

    let mut command = packet.encode();
    let written = usbfs_bulk_transfer(
        &file,
        USB_REALTIME_OUT_ENDPOINT,
        &mut command,
        options.timeout_ms,
    )?;
    if written != command.len() {
        return Err(format!(
            "short realtime command write: wrote {written} of {} bytes",
            command.len()
        ));
    }

    let mut response = [0; STATUS_PACKET_LEN];
    let read = usbfs_bulk_transfer(
        &file,
        USB_REALTIME_IN_ENDPOINT,
        &mut response,
        options.timeout_ms,
    )?;
    if read != STATUS_PACKET_LEN {
        return Err(format!(
            "short realtime status read: read {read} of {STATUS_PACKET_LEN} bytes"
        ));
    }

    StatusPacket::decode(&response).map_err(|error| format!("status decode failed: {error:?}"))
}

fn claim_usb_interface(file: &fs::File, interface: u32) -> Result<UsbInterfaceClaim<'_>, String> {
    let mut value = interface;
    let result = unsafe { ioctl(file.as_raw_fd(), USBDEVFS_CLAIMINTERFACE, &mut value) };
    if result < 0 {
        return Err(format!(
            "failed to claim USB interface {interface}: {}",
            io::Error::last_os_error()
        ));
    }

    Ok(UsbInterfaceClaim { file, interface })
}

fn usbfs_bulk_transfer(
    file: &fs::File,
    endpoint: u32,
    data: &mut [u8],
    timeout_ms: u32,
) -> Result<usize, String> {
    let len: u32 = data
        .len()
        .try_into()
        .map_err(|_| "USB transfer length does not fit in u32".to_string())?;
    let mut transfer = UsbdevfsBulkTransfer {
        ep: endpoint,
        len,
        timeout: timeout_ms,
        data: data.as_mut_ptr().cast(),
    };

    let result = unsafe { ioctl(file.as_raw_fd(), USBDEVFS_BULK, &mut transfer) };
    if result < 0 {
        return Err(format!(
            "USB bulk transfer on endpoint 0x{endpoint:02X} failed: {}",
            io::Error::last_os_error()
        ));
    }

    Ok(result as usize)
}

impl TextApiRequestWriteOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            packet_address: None,
            sequence_address: None,
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
            sequence: 1,
            request: String::new(),
        };

        let mut request = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--packet-address" => {
                    index += 1;
                    options.packet_address =
                        Some(parse_u32_arg(args.get(index), "--packet-address")?);
                }
                "--sequence-address" => {
                    index += 1;
                    options.sequence_address =
                        Some(parse_u32_arg(args.get(index), "--sequence-address")?);
                }
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    options.elf_path = PathBuf::from(path);
                }
                "--sequence" => {
                    index += 1;
                    let sequence = parse_u32_arg(args.get(index), "--sequence")?;
                    options.sequence = sequence
                        .try_into()
                        .map_err(|_| "--sequence must fit in u8".to_string())?;
                }
                "--speed" => {
                    index += 1;
                    options.jlink.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                value => {
                    if request.is_some() {
                        return Err(format!("unexpected extra request argument `{value}`"));
                    }
                    request = Some(value.to_string());
                }
            }
            index += 1;
        }

        options.request = request
            .ok_or_else(|| "write-text-api-request-jlink requires an API request".to_string())?;
        Ok(options)
    }

    fn resolve(self) -> Result<ResolvedTextApiRequestWriteOptions, String> {
        let packet_address = resolve_optional_symbol_address(
            self.packet_address,
            &self.elf_path,
            TEXT_API_REQUEST_PACKET_SYMBOL,
        )?;
        let sequence_address = resolve_optional_symbol_address(
            self.sequence_address,
            &self.elf_path,
            TEXT_API_REQUEST_SEQUENCE_SYMBOL,
        )?;

        Ok(ResolvedTextApiRequestWriteOptions {
            jlink: self.jlink,
            packet_address,
            sequence_address,
            sequence: self.sequence,
            request: self.request,
        })
    }
}

impl DriverCommandWriteOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            packet_address: None,
            sequence_address: None,
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
            sequence: 1,
            command: DriverCommand::Disable,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--packet-address" => {
                    index += 1;
                    options.packet_address =
                        Some(parse_u32_arg(args.get(index), "--packet-address")?);
                }
                "--sequence-address" => {
                    index += 1;
                    options.sequence_address =
                        Some(parse_u32_arg(args.get(index), "--sequence-address")?);
                }
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    options.elf_path = PathBuf::from(path);
                }
                "--sequence" => {
                    index += 1;
                    let sequence = parse_u32_arg(args.get(index), "--sequence")?;
                    options.sequence = sequence
                        .try_into()
                        .map_err(|_| "--sequence must fit in u8".to_string())?;
                }
                "--command" => {
                    index += 1;
                    let command = args
                        .get(index)
                        .ok_or_else(|| "--command requires a value".to_string())?;
                    options.command = parse_driver_command(command)?;
                }
                "--speed" => {
                    index += 1;
                    options.jlink.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }

    fn resolve(self) -> Result<ResolvedDriverCommandWriteOptions, String> {
        let packet_address = resolve_optional_symbol_address(
            self.packet_address,
            &self.elf_path,
            DRIVER_COMMAND_PACKET_SYMBOL,
        )?;
        let sequence_address = resolve_optional_symbol_address(
            self.sequence_address,
            &self.elf_path,
            DRIVER_COMMAND_SEQUENCE_SYMBOL,
        )?;

        Ok(ResolvedDriverCommandWriteOptions {
            jlink: self.jlink,
            packet_address,
            sequence_address,
            sequence: self.sequence,
            command: self.command,
        })
    }
}

impl CommandWriteOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            packet_address: None,
            sequence_address: None,
            elf_path: PathBuf::from(DEFAULT_ELF_PATH),
            sequence: 1,
            mode: ControlMode::Disabled,
            torque_nm: 0.0,
            velocity_rad_s: 0.0,
            position_rad: 0.0,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--packet-address" => {
                    index += 1;
                    options.packet_address =
                        Some(parse_u32_arg(args.get(index), "--packet-address")?);
                }
                "--sequence-address" => {
                    index += 1;
                    options.sequence_address =
                        Some(parse_u32_arg(args.get(index), "--sequence-address")?);
                }
                "--elf" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "--elf requires a value".to_string())?;
                    options.elf_path = PathBuf::from(path);
                }
                "--sequence" => {
                    index += 1;
                    let sequence = parse_u32_arg(args.get(index), "--sequence")?;
                    options.sequence = sequence
                        .try_into()
                        .map_err(|_| "--sequence must fit in u8".to_string())?;
                }
                "--mode" => {
                    index += 1;
                    let mode = args
                        .get(index)
                        .ok_or_else(|| "--mode requires a value".to_string())?;
                    options.mode = parse_control_mode(mode)?;
                }
                "--torque" => {
                    index += 1;
                    options.torque_nm = parse_f32_arg(args.get(index), "--torque")?;
                }
                "--velocity" => {
                    index += 1;
                    options.velocity_rad_s = parse_f32_arg(args.get(index), "--velocity")?;
                }
                "--position" => {
                    index += 1;
                    options.position_rad = parse_f32_arg(args.get(index), "--position")?;
                }
                "--speed" => {
                    index += 1;
                    options.jlink.speed_khz = parse_u32_arg(args.get(index), "--speed")?;
                }
                "--device" => {
                    index += 1;
                    let device = args
                        .get(index)
                        .ok_or_else(|| "--device requires a value".to_string())?;
                    if device != DEFAULT_DEVICE {
                        return Err(format!(
                            "unsupported device `{device}`; this helper currently supports `{DEFAULT_DEVICE}`"
                        ));
                    }
                }
                other => return Err(format!("unknown option `{other}`")),
            }
            index += 1;
        }

        Ok(options)
    }

    fn resolve(self) -> Result<ResolvedCommandWriteOptions, String> {
        let packet_address = resolve_optional_symbol_address(
            self.packet_address,
            &self.elf_path,
            COMMAND_PACKET_SYMBOL,
        )?;
        let sequence_address = resolve_optional_symbol_address(
            self.sequence_address,
            &self.elf_path,
            COMMAND_SEQUENCE_SYMBOL,
        )?;

        Ok(ResolvedCommandWriteOptions {
            jlink: self.jlink,
            packet_address,
            sequence_address,
            sequence: self.sequence,
            mode: self.mode,
            torque_nm: self.torque_nm,
            velocity_rad_s: self.velocity_rad_s,
            position_rad: self.position_rad,
        })
    }
}

fn parse_u32_arg(value: Option<&String>, flag: &str) -> Result<u32, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a value"))?;
    parse_u32(value).ok_or_else(|| format!("invalid {flag} value `{value}`"))
}

fn parse_f32_arg(value: Option<&String>, flag: &str) -> Result<f32, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a value"))?;
    value
        .parse()
        .map_err(|_| format!("invalid {flag} value `{value}`"))
}

fn parse_control_mode(value: &str) -> Result<ControlMode, String> {
    match value {
        "disabled" => Ok(ControlMode::Disabled),
        "torque" => Ok(ControlMode::Torque),
        "velocity" => Ok(ControlMode::Velocity),
        "position" => Ok(ControlMode::Position),
        "clear-faults" | "clear_faults" => Ok(ControlMode::ClearFaults),
        _ => Err(format!(
            "invalid --mode `{value}`; expected disabled, torque, velocity, position, or clear-faults"
        )),
    }
}

fn parse_driver_command(value: &str) -> Result<DriverCommand, String> {
    match value {
        "disable" => Ok(DriverCommand::Disable),
        "configure-enable" => Ok(DriverCommand::ConfigureEnable),
        _ => Err(format!(
            "invalid --command `{value}`; expected disable or configure-enable"
        )),
    }
}

fn parse_u32(value: &str) -> Option<u32> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |hex| u32::from_str_radix(hex, 16).ok(),
        )
}

fn resolve_optional_symbol_address(
    address: Option<u32>,
    path: &Path,
    symbol: &str,
) -> Result<u32, String> {
    match address {
        Some(address) => Ok(address),
        None => resolve_symbol_address(path, symbol),
    }
}

fn heap_allocator_symbols_in_elf(path: &Path) -> Result<Vec<String>, String> {
    let output = run_llvm_nm(path)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "llvm-nm failed for `{}` with status {}
stdout:
{}
stderr:
{}",
            path.display(),
            output.status,
            stdout,
            stderr
        ));
    }

    Ok(heap_allocator_symbols_in_nm_output(&stdout))
}

fn heap_allocator_symbols_in_nm_output(output: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let Some(first) = fields.next() else {
            continue;
        };
        let symbol = match (fields.next(), fields.next()) {
            (Some(_kind), Some(name)) => name,
            (None, None) => first,
            _ => continue,
        };
        if HEAP_ALLOCATOR_SYMBOLS.contains(&symbol) {
            symbols.push(symbol.to_string());
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

fn resolve_symbol_address(path: &Path, symbol: &str) -> Result<u32, String> {
    let output = run_llvm_nm(path)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "llvm-nm failed for `{}` with status {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status,
            stdout,
            stderr
        ));
    }

    parse_nm_symbol_address(&stdout, symbol)
        .ok_or_else(|| format!("symbol `{symbol}` not found in `{}`", path.display()))
}

fn run_llvm_nm(path: &Path) -> Result<std::process::Output, String> {
    match Command::new("llvm-nm").arg(path).output() {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let tool = find_rust_tool("llvm-nm")?;
            Command::new(&tool).arg(path).output().map_err(|error| {
                format!(
                    "failed to run `{}` for `{}`: {error}; pass --address to avoid symbol lookup",
                    tool.display(),
                    path.display()
                )
            })
        }
        Err(error) => Err(format!(
            "failed to run llvm-nm for `{}`: {error}; pass --address to avoid symbol lookup",
            path.display()
        )),
    }
}

fn find_rust_tool(name: &str) -> Result<PathBuf, String> {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .map_err(|error| format!("failed to run rustc --print sysroot: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustc --print sysroot failed with status {}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rustlib = Path::new(stdout.trim()).join("lib/rustlib");
    let entries = fs::read_dir(&rustlib)
        .map_err(|error| format!("failed to read `{}`: {error}", rustlib.display()))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("failed to inspect `{}` entry: {error}", rustlib.display()))?;
        let candidate = entry.path().join("bin").join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "could not find `{name}` under `{}`",
        rustlib.display()
    ))
}

fn parse_nm_symbol_address(output: &str, symbol: &str) -> Option<u32> {
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let Some(address) = fields.next() else {
            continue;
        };
        let _kind = fields.next();
        let Some(name) = fields.next() else {
            continue;
        };
        if name == symbol {
            return u32::from_str_radix(address, 16).ok();
        }
    }
    None
}

fn jlink_script(options: &JlinkOptions) -> String {
    jlink_read_script(options, BENCHMARK_PACKET_LEN)
}

fn jlink_read_script(options: &JlinkOptions, len: usize) -> String {
    format!(
        "device {}\nif SWD\nspeed {}\nconnect\nmem8 0x{:08X} {}\ng\nexit\n",
        options.device, options.speed_khz, options.address, len
    )
}

fn jlink_write_command_script(
    options: &ResolvedCommandWriteOptions,
    encoded_packet: &[u8],
) -> String {
    jlink_write_raw_packet_script(
        &options.jlink,
        options.packet_address,
        options.sequence_address,
        options.sequence,
        encoded_packet,
    )
}

fn jlink_write_raw_packet_script(
    jlink: &JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    encoded_packet: &[u8],
) -> String {
    let mut script = format!(
        "device {}\nif SWD\nspeed {}\nconnect\n",
        jlink.device, jlink.speed_khz
    );
    for (offset, byte) in encoded_packet.iter().copied().enumerate() {
        script.push_str(&format!(
            "w1 0x{:08X}, 0x{byte:02X}\n",
            packet_address + offset as u32
        ));
    }
    script.push_str(&format!(
        "w1 0x{:08X}, 0x{:02X}\ng\nexit\n",
        sequence_address, sequence
    ));
    script
}

fn write_temp_script(script: &str) -> Result<std::path::PathBuf, String> {
    let path = env::temp_dir().join(format!(
        "obot-bench-debug-{}-{}.jlink",
        std::process::id(),
        monotonic_suffix()
    ));
    fs::write(&path, script).map_err(|error| format!("failed to write {:?}: {error}", path))?;
    Ok(path)
}

fn monotonic_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

fn decode_packet_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_packet(bytes)?;
    Ok(format_run_stats_csv(DEFAULT_NAME, packet))
}

fn decode_packet_detail_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_packet(bytes)?;
    Ok(format_benchmark_detail_csv(DEFAULT_NAME, packet))
}

fn decode_status_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_status_packet(bytes)?;
    Ok(format_status_csv(DEFAULT_NAME, packet))
}

fn decode_driver_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_driver_report_packet(bytes)?;
    Ok(format_driver_csv(DEFAULT_NAME, packet))
}

fn decode_output_safety_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_output_safety_packet(bytes)?;
    Ok(format_output_safety_csv(DEFAULT_NAME, packet))
}

fn decode_bus_voltage_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_bus_voltage_packet(bytes)?;
    Ok(format_bus_voltage_csv(DEFAULT_NAME, packet))
}

fn decode_text_api_response_csv(bytes: &[u8]) -> Result<String, String> {
    let packet = decode_text_api_response_packet(bytes)?;
    Ok(format_text_api_response_csv(DEFAULT_NAME, packet))
}

fn decode_status_packet(bytes: &[u8]) -> Result<StatusPacket, String> {
    if bytes.len() != STATUS_PACKET_LEN {
        return Err(format!(
            "expected {} status bytes, got {}",
            STATUS_PACKET_LEN,
            bytes.len()
        ));
    }

    StatusPacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

fn decode_driver_report_packet(bytes: &[u8]) -> Result<DriverReportPacket, String> {
    if bytes.len() != DRIVER_REPORT_PACKET_LEN {
        return Err(format!(
            "expected {} driver report bytes, got {}",
            DRIVER_REPORT_PACKET_LEN,
            bytes.len()
        ));
    }

    DriverReportPacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

fn decode_output_safety_packet(bytes: &[u8]) -> Result<OutputSafetyPacket, String> {
    if bytes.len() != OUTPUT_SAFETY_PACKET_LEN {
        return Err(format!(
            "expected {} output safety bytes, got {}",
            OUTPUT_SAFETY_PACKET_LEN,
            bytes.len()
        ));
    }

    OutputSafetyPacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

fn decode_bus_voltage_packet(bytes: &[u8]) -> Result<BusVoltagePacket, String> {
    if bytes.len() != BUS_VOLTAGE_PACKET_LEN {
        return Err(format!(
            "expected {} bus voltage bytes, got {}",
            BUS_VOLTAGE_PACKET_LEN,
            bytes.len()
        ));
    }

    BusVoltagePacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

fn decode_text_api_response_packet(bytes: &[u8]) -> Result<TextApiResponsePacket, String> {
    if bytes.len() != TEXT_API_RESPONSE_PACKET_LEN {
        return Err(format!(
            "expected {} text API response bytes, got {}",
            TEXT_API_RESPONSE_PACKET_LEN,
            bytes.len()
        ));
    }

    TextApiResponsePacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

#[derive(Clone, Copy, Debug)]
struct DebugSnapshot {
    benchmark: BenchmarkPacket,
    status: StatusPacket,
    driver: DriverReportPacket,
    output_safety: OutputSafetyPacket,
    bus_voltage: BusVoltagePacket,
}

const SNAPSHOT_API_ENTRY_COUNT: usize = 44;

fn snapshot_api_entries(snapshot: DebugSnapshot) -> [ApiEntry<'static>; SNAPSHOT_API_ENTRY_COUNT] {
    let report = snapshot.benchmark.report;
    let status = snapshot.output_safety.status;
    let bus_sample = BusVoltageCalibration::MOTOR_HALL.convert(snapshot.bus_voltage.raw);
    let bus_allows_output = OutputGate::MOTOR_HALL.allows_output_raw(snapshot.bus_voltage.raw);
    let combined_max_cycles = FAST_LOOPS_PER_MAIN_LOOP * report.max_fast_loop_cycles() as u64
        + report.max_main_loop_cycles() as u64;
    let combined_mean_milli_cycles = FAST_LOOPS_PER_MAIN_LOOP
        * report.mean_fast_loop_cycles_milli_cycles()
        + report.mean_main_loop_cycles_milli_cycles();
    let combined_mean_remaining_milli_cycles =
        CYCLES_PER_100_US as i64 * 1_000 - combined_mean_milli_cycles as i64;

    [
        ApiEntry::new("api_length", ApiValue::U16(SNAPSHOT_API_ENTRY_COUNT as u16)),
        ApiEntry::new("cpu_frequency", ApiValue::U32(170_000_000)),
        ApiEntry::new("messages_version", ApiValue::Str("3.3")),
        ApiEntry::new("t_exec_fastloop", ApiValue::U32(report.t_exec_fastloop())),
        ApiEntry::new("t_exec_mainloop", ApiValue::U32(report.t_exec_mainloop())),
        ApiEntry::new(
            "t_period_fastloop",
            ApiValue::U32(report.t_period_fastloop()),
        ),
        ApiEntry::new(
            "t_period_mainloop",
            ApiValue::U32(report.t_period_mainloop()),
        ),
        ApiEntry::new(
            "max_fast_loop_cycles",
            ApiValue::U32(report.max_fast_loop_cycles()),
        ),
        ApiEntry::new(
            "max_fast_loop_period",
            ApiValue::U32(report.max_fast_loop_period_cycles()),
        ),
        ApiEntry::new(
            "fast_max_load_percent",
            ApiValue::F32(percent_f32(
                report.max_fast_loop_cycles() as u64,
                report.max_fast_loop_period_cycles() as u64,
            )),
        ),
        ApiEntry::new(
            "fast_max_remaining_cycles",
            ApiValue::I32(
                report.max_fast_loop_period_cycles() as i32 - report.max_fast_loop_cycles() as i32,
            ),
        ),
        ApiEntry::new(
            "max_main_loop_cycles",
            ApiValue::U32(report.max_main_loop_cycles()),
        ),
        ApiEntry::new(
            "max_main_loop_period",
            ApiValue::U32(report.max_main_loop_period_cycles()),
        ),
        ApiEntry::new(
            "main_max_load_percent",
            ApiValue::F32(percent_f32(
                report.max_main_loop_cycles() as u64,
                report.max_main_loop_period_cycles() as u64,
            )),
        ),
        ApiEntry::new(
            "main_max_remaining_cycles",
            ApiValue::I32(
                report.max_main_loop_period_cycles() as i32 - report.max_main_loop_cycles() as i32,
            ),
        ),
        ApiEntry::new(
            "combined_max_cycles",
            ApiValue::U32(combined_max_cycles as u32),
        ),
        ApiEntry::new(
            "combined_max_load_percent",
            ApiValue::F32(percent_f32(combined_max_cycles, CYCLES_PER_100_US)),
        ),
        ApiEntry::new(
            "combined_max_remaining_cycles",
            ApiValue::I32(CYCLES_PER_100_US as i32 - combined_max_cycles as i32),
        ),
        ApiEntry::new(
            "mean_fast_loop_cycles",
            ApiValue::F32(milli_cycles_to_f32(
                report.mean_fast_loop_cycles_milli_cycles(),
            )),
        ),
        ApiEntry::new(
            "mean_fast_loop_period",
            ApiValue::F32(milli_cycles_to_f32(
                report.mean_fast_loop_period_milli_cycles(),
            )),
        ),
        ApiEntry::new(
            "mean_main_loop_cycles",
            ApiValue::F32(milli_cycles_to_f32(
                report.mean_main_loop_cycles_milli_cycles(),
            )),
        ),
        ApiEntry::new(
            "mean_main_loop_period",
            ApiValue::F32(milli_cycles_to_f32(
                report.mean_main_loop_period_milli_cycles(),
            )),
        ),
        ApiEntry::new(
            "combined_mean_cycles",
            ApiValue::F32(milli_cycles_to_f32(combined_mean_milli_cycles)),
        ),
        ApiEntry::new(
            "combined_mean_load_percent",
            ApiValue::F32(milli_percent_f32(
                combined_mean_milli_cycles,
                CYCLES_PER_100_US,
            )),
        ),
        ApiEntry::new(
            "combined_mean_remaining_cycles",
            ApiValue::F32(signed_milli_cycles_to_f32(
                combined_mean_remaining_milli_cycles,
            )),
        ),
        ApiEntry::new(
            "fault",
            ApiValue::Str(format_fault(snapshot.status.state.fault)),
        ),
        ApiEntry::new("torque_nm", ApiValue::F32(snapshot.status.state.torque_nm)),
        ApiEntry::new(
            "velocity_rad_s",
            ApiValue::F32(snapshot.status.state.velocity_rad_s),
        ),
        ApiEntry::new(
            "position_rad",
            ApiValue::F32(snapshot.status.state.position_rad),
        ),
        ApiEntry::new("output_allowed", ApiValue::Bool(status.output_allowed)),
        ApiEntry::new("command_blocked", ApiValue::Bool(status.command_blocked)),
        ApiEntry::new("bus_blocked", ApiValue::Bool(status.bus_blocked)),
        ApiEntry::new(
            "driver_not_enabled",
            ApiValue::Bool(status.driver_not_enabled),
        ),
        ApiEntry::new(
            "driver_fault_latched",
            ApiValue::Bool(status.driver_fault_latched),
        ),
        ApiEntry::new(
            "controller_faulted",
            ApiValue::Bool(status.controller_faulted),
        ),
        ApiEntry::new("host_timed_out", ApiValue::Bool(status.host_timed_out)),
        ApiEntry::new("bus_voltage_raw", ApiValue::U16(snapshot.bus_voltage.raw)),
        ApiEntry::new("bus_voltage_volts", ApiValue::F32(bus_sample.volts)),
        ApiEntry::new("bus_allows_output", ApiValue::Bool(bus_allows_output)),
        ApiEntry::new(
            "driver_configured",
            ApiValue::Bool(snapshot.driver.configured),
        ),
        ApiEntry::new(
            "verify_error_mask",
            ApiValue::U16(snapshot.driver.verify_error_mask),
        ),
        ApiEntry::new(
            "transfer_error_mask",
            ApiValue::U16(snapshot.driver.transfer_error_mask),
        ),
        ApiEntry::new(
            "status_before",
            ApiValue::U32(snapshot.driver.status_before),
        ),
        ApiEntry::new("status_after", ApiValue::U32(snapshot.driver.status_after)),
    ]
}

fn format_api_dispatch_error(error: ApiDispatchError) -> String {
    match error {
        ApiDispatchError::Parse(error) => format!("parse failed: {error:?}"),
        ApiDispatchError::UnknownName => "unknown API variable".to_string(),
        ApiDispatchError::ReadOnly => "API variable is read-only".to_string(),
        ApiDispatchError::NameIndexOutOfRange => "API variable index out of range".to_string(),
        ApiDispatchError::ResponseTooLong => "API response buffer too small".to_string(),
    }
}

fn milli_cycles_to_f32(value: u64) -> f32 {
    value as f32 / 1_000.0
}

fn signed_milli_cycles_to_f32(value: i64) -> f32 {
    value as f32 / 1_000.0
}

fn percent_f32(numerator_cycles: u64, denominator_cycles: u64) -> f32 {
    if denominator_cycles == 0 {
        return f32::NAN;
    }

    numerator_cycles as f32 * 100.0 / denominator_cycles as f32
}

fn milli_percent_f32(numerator_milli_cycles: u64, denominator_cycles: u64) -> f32 {
    if denominator_cycles == 0 {
        return f32::NAN;
    }

    numerator_milli_cycles as f32 * 100.0 / (denominator_cycles as f32 * 1_000.0)
}

fn format_snapshot_csv(name: &str, snapshot: DebugSnapshot) -> String {
    let report = snapshot.benchmark.report;
    let status = snapshot.output_safety.status;
    let bus_sample = BusVoltageCalibration::MOTOR_HALL.convert(snapshot.bus_voltage.raw);
    let bus_allows_output = OutputGate::MOTOR_HALL.allows_output_raw(snapshot.bus_voltage.raw);
    let combined_max_cycles = FAST_LOOPS_PER_MAIN_LOOP * report.max_fast_loop_cycles() as u64
        + report.max_main_loop_cycles() as u64;
    let combined_max_remaining_cycles = CYCLES_PER_100_US as i64 - combined_max_cycles as i64;
    let combined_mean_milli_cycles = FAST_LOOPS_PER_MAIN_LOOP
        * report.mean_fast_loop_cycles_milli_cycles()
        + report.mean_main_loop_cycles_milli_cycles();
    let combined_mean_remaining_milli_cycles =
        CYCLES_PER_100_US as i64 * 1_000 - combined_mean_milli_cycles as i64;

    format!(
        "name, benchmark_sequence, status_sequence, driver_sequence, output_safety_sequence, bus_voltage_sequence, max_fast_loop_cycles, max_fast_loop_period, fast_max_load_percent, fast_max_remaining_cycles, max_main_loop_cycles, max_main_loop_period, main_max_load_percent, main_max_remaining_cycles, combined_max_cycles, combined_max_load_percent, combined_max_remaining_cycles, mean_fast_loop_cycles, mean_main_loop_cycles, combined_mean_cycles, combined_mean_load_percent, combined_mean_remaining_cycles, fault, output_allowed, command_blocked, bus_blocked, driver_not_enabled, driver_fault_latched, controller_faulted, host_timed_out, bus_voltage_raw, bus_voltage_volts, bus_allows_output, driver_configured, verify_error_mask, transfer_error_mask, status_before, status_after, torque_nm, velocity_rad_s, position_rad\n{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {:.3}, {}, {}, 0x{:04X}, 0x{:04X}, 0x{:08X}, 0x{:08X}, {}, {}, {}\n",
        name,
        snapshot.benchmark.sequence,
        snapshot.status.sequence,
        snapshot.driver.sequence,
        snapshot.output_safety.sequence,
        snapshot.bus_voltage.sequence,
        report.max_fast_loop_cycles(),
        report.max_fast_loop_period_cycles(),
        format_percent(
            report.max_fast_loop_cycles() as u64,
            report.max_fast_loop_period_cycles() as u64,
        ),
        report.max_fast_loop_period_cycles() as i64 - report.max_fast_loop_cycles() as i64,
        report.max_main_loop_cycles(),
        report.max_main_loop_period_cycles(),
        format_percent(
            report.max_main_loop_cycles() as u64,
            report.max_main_loop_period_cycles() as u64,
        ),
        report.max_main_loop_period_cycles() as i64 - report.max_main_loop_cycles() as i64,
        combined_max_cycles,
        format_percent(combined_max_cycles, CYCLES_PER_100_US),
        combined_max_remaining_cycles,
        format_milli_cycles(report.mean_fast_loop_cycles_milli_cycles()),
        format_milli_cycles(report.mean_main_loop_cycles_milli_cycles()),
        format_milli_cycles(combined_mean_milli_cycles),
        format_milli_percent(combined_mean_milli_cycles, CYCLES_PER_100_US),
        format_signed_milli_cycles(combined_mean_remaining_milli_cycles),
        format_fault(snapshot.status.state.fault),
        status.output_allowed,
        status.command_blocked,
        status.bus_blocked,
        status.driver_not_enabled,
        status.driver_fault_latched,
        status.controller_faulted,
        status.host_timed_out,
        snapshot.bus_voltage.raw,
        bus_sample.volts,
        bus_allows_output,
        snapshot.driver.configured,
        snapshot.driver.verify_error_mask,
        snapshot.driver.transfer_error_mask,
        snapshot.driver.status_before,
        snapshot.driver.status_after,
        snapshot.status.state.torque_nm,
        snapshot.status.state.velocity_rad_s,
        snapshot.status.state.position_rad,
    )
}

fn format_status_csv(name: &str, packet: StatusPacket) -> String {
    format!(
        "name, sequence, fault, torque_nm, velocity_rad_s, position_rad\n{}, {}, {}, {}, {}, {}\n",
        name,
        packet.sequence,
        format_fault(packet.state.fault),
        packet.state.torque_nm,
        packet.state.velocity_rad_s,
        packet.state.position_rad,
    )
}

fn format_driver_csv(name: &str, packet: DriverReportPacket) -> String {
    format!(
        "name, sequence, configured, verify_error_mask, transfer_error_mask, status_before, status_after\n{}, {}, {}, 0x{:04X}, 0x{:04X}, 0x{:08X}, 0x{:08X}\n",
        name,
        packet.sequence,
        packet.configured,
        packet.verify_error_mask,
        packet.transfer_error_mask,
        packet.status_before,
        packet.status_after,
    )
}

fn format_output_safety_csv(name: &str, packet: OutputSafetyPacket) -> String {
    let status = packet.status;
    format!(
        "name, sequence, output_allowed, command_blocked, bus_blocked, driver_not_enabled, driver_fault_latched, controller_faulted, host_timed_out\n{}, {}, {}, {}, {}, {}, {}, {}, {}\n",
        name,
        packet.sequence,
        status.output_allowed,
        status.command_blocked,
        status.bus_blocked,
        status.driver_not_enabled,
        status.driver_fault_latched,
        status.controller_faulted,
        status.host_timed_out,
    )
}

fn format_bus_voltage_csv(name: &str, packet: BusVoltagePacket) -> String {
    let sample = BusVoltageCalibration::MOTOR_HALL.convert(packet.raw);
    let bus_allows_output = OutputGate::MOTOR_HALL.allows_output_raw(packet.raw);
    format!(
        "name, sequence, bus_voltage_raw, bus_voltage_volts, bus_allows_output\n{}, {}, {}, {:.3}, {}\n",
        name, packet.sequence, packet.raw, sample.volts, bus_allows_output,
    )
}

fn format_text_api_response_csv(name: &str, packet: TextApiResponsePacket) -> String {
    let response = std::str::from_utf8(packet.payload()).unwrap_or("<invalid-utf8>");
    format!(
        "name, sequence, status, response\n{}, {}, {}, {}\n",
        name,
        packet.sequence,
        format_text_api_status(packet.status),
        response,
    )
}

fn format_text_api_status(status: TextApiResponseStatus) -> &'static str {
    match status {
        TextApiResponseStatus::Ok => "ok",
        TextApiResponseStatus::ParseError => "parse_error",
        TextApiResponseStatus::UnknownName => "unknown_name",
        TextApiResponseStatus::ReadOnly => "read_only",
        TextApiResponseStatus::NameIndexOutOfRange => "name_index_out_of_range",
        TextApiResponseStatus::ResponseTooLong => "response_too_long",
        TextApiResponseStatus::InvalidUtf8 => "invalid_utf8",
    }
}

fn format_fault(fault: Option<obot_core::Fault>) -> &'static str {
    match fault {
        None => "none",
        Some(obot_core::Fault::CommandNotFinite) => "command_not_finite",
        Some(obot_core::Fault::TorqueLimit) => "torque_limit",
        Some(obot_core::Fault::VelocityLimit) => "velocity_limit",
        Some(obot_core::Fault::PositionLimit) => "position_limit",
    }
}

fn decode_packet(bytes: &[u8]) -> Result<BenchmarkPacket, String> {
    if bytes.len() != BENCHMARK_PACKET_LEN {
        return Err(format!(
            "expected {} bytes, got {}",
            BENCHMARK_PACKET_LEN,
            bytes.len()
        ));
    }

    BenchmarkPacket::decode(bytes).map_err(|error| format!("decode failed: {error:?}"))
}

fn format_usb_run_stats_csv(name: &str, stats: UsbRunStats) -> String {
    format!(
        "name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period\n{}, {}, {}, {}, {}, {}, {}, {}, {}\n",
        name,
        stats.max_fast_loop_cycles,
        stats.max_fast_loop_period,
        stats.max_main_loop_cycles,
        stats.max_main_loop_period,
        format_milli_cycles(stats.mean_fast_loop_cycles_milli),
        format_milli_cycles(stats.mean_fast_loop_period_milli),
        format_milli_cycles(stats.mean_main_loop_cycles_milli),
        format_milli_cycles(stats.mean_main_loop_period_milli),
    )
}

fn format_run_stats_csv(name: &str, packet: BenchmarkPacket) -> String {
    let report = packet.report;
    format!(
        "name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period\n{}, {}, {}, {}, {}, {}, {}, {}, {}\n",
        name,
        report.max_fast_loop_cycles(),
        report.max_fast_loop_period_cycles(),
        report.max_main_loop_cycles(),
        report.max_main_loop_period_cycles(),
        format_milli_cycles(report.mean_fast_loop_cycles_milli_cycles()),
        format_milli_cycles(report.mean_fast_loop_period_milli_cycles()),
        format_milli_cycles(report.mean_main_loop_cycles_milli_cycles()),
        format_milli_cycles(report.mean_main_loop_period_milli_cycles()),
    )
}

fn format_benchmark_detail_csv(name: &str, packet: BenchmarkPacket) -> String {
    let report = packet.report;
    let rows = [
        ("fast_period", report.fast.period),
        ("fast_execution", report.fast.execution),
        ("main_period", report.main.period),
        ("main_execution", report.main.execution),
    ];

    let mut output =
        "name, sequence, metric, samples, last_cycles, max_cycles, mean_cycles\n".to_string();
    for (metric, stats) in rows {
        output.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {}\n",
            name,
            packet.sequence,
            metric,
            stats.samples,
            stats.last_cycles,
            stats.max_cycles,
            format_milli_cycles(stats.mean_milli_cycles),
        ));
    }
    output
}

fn format_milli_cycles(value: u64) -> String {
    let whole = value / 1_000;
    let fraction = value % 1_000;
    if fraction == 0 {
        return whole.to_string();
    }

    let mut out = format!("{whole}.{fraction:03}");
    while out.ends_with('0') {
        out.pop();
    }
    out
}

fn format_signed_milli_cycles(value: i64) -> String {
    if value < 0 {
        format!("-{}", format_milli_cycles(value.unsigned_abs()))
    } else {
        format_milli_cycles(value as u64)
    }
}

fn format_percent(numerator_cycles: u64, denominator_cycles: u64) -> String {
    if denominator_cycles == 0 {
        return "nan".to_string();
    }

    format!(
        "{:.2}",
        numerator_cycles as f64 * 100.0 / denominator_cycles as f64
    )
}

fn format_milli_percent(numerator_milli_cycles: u64, denominator_cycles: u64) -> String {
    if denominator_cycles == 0 {
        return "nan".to_string();
    }

    format!(
        "{:.2}",
        numerator_milli_cycles as f64 * 100.0 / (denominator_cycles as f64 * 1_000.0)
    )
}

fn parse_hex_bytes(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for token in
        input.split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == ':' || ch == '=')
    {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let token = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
            .unwrap_or(token);
        if token.len() % 2 != 0 {
            return Err(format!("hex token `{token}` has an odd number of digits"));
        }
        for chunk_start in (0..token.len()).step_by(2) {
            let byte =
                u8::from_str_radix(&token[chunk_start..chunk_start + 2], 16).map_err(|_| {
                    format!(
                        "invalid hex byte `{}`",
                        &token[chunk_start..chunk_start + 2]
                    )
                })?;
            out.push(byte);
        }
    }
    Ok(out)
}

fn parse_jlink_mem8_output(output: &str, expected_len: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(expected_len);
    for line in output.lines() {
        let Some((_, byte_text)) = line.split_once('=') else {
            continue;
        };
        for token in byte_text.split_ascii_whitespace() {
            let token = token.trim_matches(|ch: char| !ch.is_ascii_hexdigit());
            if token.len() != 2 {
                continue;
            }
            if let Ok(byte) = u8::from_str_radix(token, 16) {
                bytes.push(byte);
                if bytes.len() == expected_len {
                    return Ok(bytes);
                }
            }
        }
    }

    Err(format!(
        "J-Link output contained {} benchmark bytes, expected {}",
        bytes.len(),
        expected_len
    ))
}

fn usage() -> String {
    format!(
        "usage:
  obot-bench-debug decode-hex <{} benchmark bytes as hex>
  obot-bench-debug decode-file <path-to-raw-{}-byte-benchmark-packet>
  obot-bench-debug decode-detail-hex <{} benchmark bytes as hex>
  obot-bench-debug decode-detail-file <path-to-raw-{}-byte-benchmark-packet>
  obot-bench-debug decode-status-hex <{} status bytes as hex>
  obot-bench-debug decode-driver-hex <{} driver report bytes as hex>
  obot-bench-debug decode-output-safety-hex <{} output safety bytes as hex>
  obot-bench-debug decode-bus-voltage-hex <{} bus voltage bytes as hex>
  obot-bench-debug decode-text-api-response-hex <{} text API response bytes as hex>
  obot-bench-debug jlink-script [--address 0x20000000] [--speed 4000]
  obot-bench-debug read-jlink [--address 0x20000000] [--speed 4000]
  obot-bench-debug read-jlink-detail [--address 0x20000000] [--speed 4000]
  obot-bench-debug run-stats-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address 0x20000000] [--speed 4000]
  obot-bench-debug run-stats-usb [samples] [--samples N] [--dev /dev/bus/usb/<bus>/<dev>] [--timeout-ms N]
  obot-bench-debug verify-no-heap [--elf target/thumbv7em-none-eabihf/release/obot-g474]
  obot-bench-debug read-status-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <status-packet-address>] [--speed 4000]
  obot-bench-debug read-driver-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <driver-report-address>] [--speed 4000]
  obot-bench-debug read-output-safety-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <output-safety-address>] [--speed 4000]
  obot-bench-debug read-bus-voltage-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <bus-voltage-address>] [--speed 4000]
  obot-bench-debug snapshot-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--speed 4000]
  obot-bench-debug api-snapshot-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--speed 4000] <api-request>
  obot-bench-debug write-text-api-request-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--packet-address <text-api-request-address>] [--sequence-address <text-api-request-sequence-address>] [--sequence N] <api-request>
  obot-bench-debug read-text-api-response-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <text-api-response-address>] [--speed 4000]
  obot-bench-debug read-text-api-usb [--dev /dev/bus/usb/<bus>/<dev>] [--timeout-ms N] <api-request>
  obot-bench-debug write-command-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--packet-address <command-packet-address>] [--sequence-address <command-sequence-address>] [--sequence N] [--mode disabled|torque|velocity|position|clear-faults] [--torque Nm] [--velocity rad_s] [--position rad]
  obot-bench-debug write-command-usb [--dev /dev/bus/usb/<bus>/<dev>] [--sequence N] [--mode disabled|torque|velocity|position|clear-faults] [--torque Nm] [--velocity rad_s] [--position rad] [--timeout-ms N]
  obot-bench-debug write-driver-command-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--packet-address <driver-command-packet-address>] [--sequence-address <driver-command-sequence-address>] [--sequence N] [--command disable|configure-enable]
",
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        STATUS_PACKET_LEN,
        DRIVER_REPORT_PACKET_LEN,
        OUTPUT_SAFETY_PACKET_LEN,
        BUS_VOLTAGE_PACKET_LEN,
        TEXT_API_RESPONSE_PACKET_LEN
    )
}

#[allow(dead_code)]
fn read_file(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    fs::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use obot_core::benchmark::{BenchmarkReport, CycleStatsSnapshot, LoopBenchmarkSnapshot};

    fn sample_packet() -> BenchmarkPacket {
        BenchmarkPacket {
            sequence: 9,
            report: BenchmarkReport {
                fast: LoopBenchmarkSnapshot {
                    period: CycleStatsSnapshot {
                        samples: 10,
                        last_cycles: 3_398,
                        max_cycles: 3_416,
                        mean_milli_cycles: 3_397_560,
                    },
                    execution: CycleStatsSnapshot {
                        samples: 11,
                        last_cycles: 709,
                        max_cycles: 710,
                        mean_milli_cycles: 708_965,
                    },
                },
                main: LoopBenchmarkSnapshot {
                    period: CycleStatsSnapshot {
                        samples: 12,
                        last_cycles: 17_000,
                        max_cycles: 17_045,
                        mean_milli_cycles: 16_999_800,
                    },
                    execution: CycleStatsSnapshot {
                        samples: 13,
                        last_cycles: 3_555,
                        max_cycles: 6_445,
                        mean_milli_cycles: 3_555_490,
                    },
                },
            },
        }
    }

    #[test]
    fn formats_milli_cycles_without_unnecessary_trailing_zeroes() {
        assert_eq!(format_milli_cycles(708_965), "708.965");
        assert_eq!(format_milli_cycles(17_000_000), "17000");
        assert_eq!(format_milli_cycles(1_250), "1.25");
    }

    #[test]
    fn prints_motor_util_run_stats_shape() {
        let output = format_run_stats_csv("rust", sample_packet());

        assert_eq!(
            output,
            "name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period\nrust, 710, 3416, 6445, 17045, 708.965, 3397.56, 3555.49, 16999.8\n"
        );
    }

    #[test]
    fn prints_detailed_benchmark_shape() {
        let output = format_benchmark_detail_csv("rust", sample_packet());

        assert_eq!(
            output,
            "name, sequence, metric, samples, last_cycles, max_cycles, mean_cycles\nrust, 9, fast_period, 10, 3398, 3416, 3397.56\nrust, 9, fast_execution, 11, 709, 710, 708.965\nrust, 9, main_period, 12, 17000, 17045, 16999.8\nrust, 9, main_execution, 13, 3555, 6445, 3555.49\n"
        );
    }

    #[test]
    fn parses_hex_bytes_from_compact_or_spaced_input() {
        assert_eq!(
            parse_hex_bytes("0x0102 03, 04:05").unwrap(),
            [1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn parses_jlink_mem8_output() {
        let encoded = sample_packet().encode();
        let mut output = String::new();
        for (row, chunk) in encoded.chunks(16).enumerate() {
            output.push_str(&format!("{:08X} =", DEFAULT_ADDRESS + (row * 16) as u32));
            for byte in chunk {
                output.push_str(&format!(" {byte:02X}"));
            }
            output.push('\n');
        }

        assert_eq!(
            parse_jlink_mem8_output(&output, BENCHMARK_PACKET_LEN).unwrap(),
            encoded
        );
    }

    #[test]
    fn builds_jlink_script_for_exported_symbol_address() {
        let script = jlink_script(&JlinkOptions {
            address: DEFAULT_ADDRESS,
            speed_khz: DEFAULT_SPEED_KHZ,
            device: DEFAULT_DEVICE,
        });

        assert!(script.contains("device STM32G474RE\n"));
        assert!(script.contains("mem8 0x20000000 81\n"));
    }

    #[test]
    fn parses_nm_symbol_address() {
        let output = "\
08000000 T Reset\n20000020 B OBOT_BENCHMARK_PACKET\n20000071 B OBOT_COMMAND_PACKET\n";

        assert_eq!(
            parse_nm_symbol_address(output, BENCHMARK_PACKET_SYMBOL),
            Some(0x2000_0020)
        );
    }

    #[test]
    fn parses_elf_only_options() {
        let options =
            ElfOnlyOptions::parse(&["--elf".to_string(), "target/custom.elf".to_string()]).unwrap();

        assert_eq!(options.elf_path, PathBuf::from("target/custom.elf"));
    }

    #[test]
    fn detects_heap_allocator_symbols_in_nm_output() {
        let output = "\
08000000 T Reset
00000000 T __rust_alloc
00000000 U __rust_dealloc
20000020 B OBOT_BENCHMARK_PACKET
";

        assert_eq!(
            heap_allocator_symbols_in_nm_output(output),
            ["__rust_alloc".to_string(), "__rust_dealloc".to_string()]
        );
    }

    #[test]
    fn parses_symbol_read_options() {
        let options = SymbolReadOptions::parse(&[
            "--elf".to_string(),
            "target/custom.elf".to_string(),
            "--speed".to_string(),
            "1000".to_string(),
        ])
        .unwrap();

        assert_eq!(options.address, None);
        assert_eq!(options.elf_path, PathBuf::from("target/custom.elf"));
        assert_eq!(options.jlink.speed_khz, 1_000);
    }

    #[test]
    fn explicit_symbol_read_address_overrides_elf_lookup() {
        let options = SymbolReadOptions::parse(&[
            "--address".to_string(),
            "0x20000020".to_string(),
            "--elf".to_string(),
            "does-not-need-to-exist".to_string(),
        ])
        .unwrap();

        let jlink = options.resolve(BENCHMARK_PACKET_SYMBOL).unwrap();

        assert_eq!(jlink.address, 0x2000_0020);
    }

    #[test]
    fn formats_status_packet_csv() {
        let output = format_status_csv(
            "rust",
            StatusPacket {
                sequence: 3,
                state: obot_core::MotorState {
                    torque_nm: 1.25,
                    velocity_rad_s: 0.0,
                    position_rad: -0.5,
                    fault: Some(obot_core::Fault::TorqueLimit),
                },
            },
        );

        assert_eq!(
            output,
            "name, sequence, fault, torque_nm, velocity_rad_s, position_rad\nrust, 3, torque_limit, 1.25, 0, -0.5\n"
        );
    }

    #[test]
    fn formats_driver_report_csv() {
        let output = format_driver_csv(
            "rust",
            DriverReportPacket {
                sequence: 4,
                configured: false,
                verify_error_mask: 0x0012,
                transfer_error_mask: 0x0040,
                status_before: 0xAABB_CCDD,
                status_after: 0x1122_3344,
            },
        );

        assert_eq!(
            output,
            "name, sequence, configured, verify_error_mask, transfer_error_mask, status_before, status_after\nrust, 4, false, 0x0012, 0x0040, 0xAABBCCDD, 0x11223344\n"
        );
    }

    #[test]
    fn formats_output_safety_csv() {
        let output = format_output_safety_csv(
            "rust",
            OutputSafetyPacket {
                sequence: 5,
                status: obot_core::output::OutputSafetyStatus {
                    output_allowed: false,
                    command_blocked: true,
                    bus_blocked: true,
                    driver_not_enabled: true,
                    driver_fault_latched: false,
                    controller_faulted: true,
                    host_timed_out: true,
                },
            },
        );

        assert_eq!(
            output,
            "name, sequence, output_allowed, command_blocked, bus_blocked, driver_not_enabled, driver_fault_latched, controller_faulted, host_timed_out\nrust, 5, false, true, true, true, false, true, true\n"
        );
    }

    #[test]
    fn formats_bus_voltage_csv() {
        let output = format_bus_voltage_csv(
            "rust",
            BusVoltagePacket {
                sequence: 6,
                raw: OutputGate::MOTOR_HALL.min_raw,
            },
        );

        assert_eq!(
            output,
            "name, sequence, bus_voltage_raw, bus_voltage_volts, bus_allows_output\nrust, 6, 1963, 8.000, true\n"
        );
    }

    #[test]
    fn formats_text_api_response_csv() {
        let output = format_text_api_response_csv(
            "rust",
            TextApiResponsePacket::new(7, TextApiResponseStatus::Ok, b"435.633").unwrap(),
        );

        assert_eq!(
            output,
            "name, sequence, status, response\nrust, 7, ok, 435.633\n"
        );
    }

    #[test]
    fn formats_combined_snapshot_csv() {
        let output = format_snapshot_csv(
            "rust",
            DebugSnapshot {
                benchmark: sample_packet(),
                status: StatusPacket {
                    sequence: 3,
                    state: obot_core::MotorState {
                        torque_nm: 1.25,
                        velocity_rad_s: 0.0,
                        position_rad: -0.5,
                        fault: Some(obot_core::Fault::TorqueLimit),
                    },
                },
                driver: DriverReportPacket {
                    sequence: 4,
                    configured: false,
                    verify_error_mask: 0x0012,
                    transfer_error_mask: 0x0040,
                    status_before: 0xAABB_CCDD,
                    status_after: 0x1122_3344,
                },
                output_safety: OutputSafetyPacket {
                    sequence: 5,
                    status: obot_core::output::OutputSafetyStatus {
                        output_allowed: false,
                        command_blocked: true,
                        bus_blocked: true,
                        driver_not_enabled: true,
                        driver_fault_latched: false,
                        controller_faulted: true,
                        host_timed_out: true,
                    },
                },
                bus_voltage: BusVoltagePacket {
                    sequence: 6,
                    raw: OutputGate::MOTOR_HALL.min_raw,
                },
            },
        );

        assert!(output.starts_with("name, benchmark_sequence, status_sequence"));
        assert!(output.contains("combined_mean_load_percent"));
        assert!(output.contains(", combined_max_remaining_cycles,"));
        assert!(output.contains("rust, 9, 3, 4, 5, 6, 710, 3416, 20.78, 2706, 6445, 17045, 37.81, 10600, 9995, 58.79, 7005, 708.965, 3555.49, 7100.315, 41.77, 9899.685, torque_limit, false, true, true, true, false, true, true, 1963, 8.000, true, false, 0x0012, 0x0040, 0xAABBCCDD, 0x11223344, 1.25, 0, -0.5\n"));
    }

    #[test]
    fn exposes_snapshot_values_through_text_api_catalog() {
        let entries = snapshot_api_entries(DebugSnapshot {
            benchmark: sample_packet(),
            status: StatusPacket {
                sequence: 3,
                state: obot_core::MotorState {
                    torque_nm: 1.25,
                    velocity_rad_s: 0.0,
                    position_rad: -0.5,
                    fault: Some(obot_core::Fault::TorqueLimit),
                },
            },
            driver: DriverReportPacket {
                sequence: 4,
                configured: false,
                verify_error_mask: 0x0012,
                transfer_error_mask: 0x0040,
                status_before: 0xAABB_CCDD,
                status_after: 0x1122_3344,
            },
            output_safety: OutputSafetyPacket {
                sequence: 5,
                status: obot_core::output::OutputSafetyStatus {
                    output_allowed: false,
                    command_blocked: true,
                    bus_blocked: true,
                    driver_not_enabled: true,
                    driver_fault_latched: false,
                    controller_faulted: true,
                    host_timed_out: true,
                },
            },
            bus_voltage: BusVoltagePacket {
                sequence: 6,
                raw: OutputGate::MOTOR_HALL.min_raw,
            },
        });
        let catalog = ApiCatalog::new(&entries);
        let mut response = [0; 96];

        assert_eq!(
            catalog.dispatch("api_name=0", &mut response),
            Ok("api_length")
        );
        assert_eq!(catalog.dispatch("api_length", &mut response), Ok("44"));
        assert_eq!(
            catalog.dispatch("api_name=7", &mut response),
            Ok("max_fast_loop_cycles")
        );
        assert_eq!(
            catalog.dispatch("t_exec_fastloop", &mut response),
            Ok("709")
        );
        assert_eq!(
            catalog.dispatch("t_period_mainloop", &mut response),
            Ok("17000")
        );
        assert_eq!(
            catalog.dispatch("max_fast_loop_cycles", &mut response),
            Ok("710")
        );
        assert_eq!(catalog.dispatch("fault", &mut response), Ok("torque_limit"));
        assert_eq!(
            catalog.dispatch("bus_allows_output", &mut response),
            Ok("true")
        );
        assert_eq!(
            catalog.dispatch("verify_error_mask", &mut response),
            Ok("18")
        );
    }

    #[test]
    fn parses_api_snapshot_options() {
        let options = ApiSnapshotOptions::parse(&[
            "--elf".to_string(),
            "target/custom.elf".to_string(),
            "--speed".to_string(),
            "1000".to_string(),
            "mean_fast_loop_cycles".to_string(),
        ])
        .unwrap();

        assert_eq!(options.symbols.elf_path, PathBuf::from("target/custom.elf"));
        assert_eq!(options.symbols.jlink.speed_khz, 1_000);
        assert_eq!(options.request, "mean_fast_loop_cycles");
    }

    #[test]
    fn snapshot_jlink_rejects_single_address_override() {
        let error = snapshot_jlink_command(&["--address".to_string(), "0x20000020".to_string()])
            .unwrap_err();

        assert!(error.contains("reads multiple symbols"));
    }

    #[test]
    fn parses_driver_command_write_options() {
        let options = DriverCommandWriteOptions::parse(&[
            "--packet-address".to_string(),
            "0x20000090".to_string(),
            "--sequence-address".to_string(),
            "0x20000092".to_string(),
            "--sequence".to_string(),
            "8".to_string(),
            "--command".to_string(),
            "configure-enable".to_string(),
        ])
        .unwrap();

        assert_eq!(options.packet_address, Some(0x2000_0090));
        assert_eq!(options.sequence_address, Some(0x2000_0092));
        assert_eq!(options.sequence, 8);
        assert_eq!(options.command, DriverCommand::ConfigureEnable);
    }

    #[test]
    fn parses_text_api_request_write_options() {
        let options = TextApiRequestWriteOptions::parse(&[
            "--packet-address".to_string(),
            "0x20000090".to_string(),
            "--sequence-address".to_string(),
            "0x200000d2".to_string(),
            "--sequence".to_string(),
            "9".to_string(),
            "mean_fast_loop_cycles".to_string(),
        ])
        .unwrap();

        assert_eq!(options.packet_address, Some(0x2000_0090));
        assert_eq!(options.sequence_address, Some(0x2000_00d2));
        assert_eq!(options.sequence, 9);
        assert_eq!(options.request, "mean_fast_loop_cycles");
    }

    #[test]
    fn parses_text_api_usb_options() {
        let options = TextApiUsbOptions::parse(&[
            "--dev".to_string(),
            "/dev/bus/usb/001/043".to_string(),
            "--timeout-ms".to_string(),
            "250".to_string(),
            "api_length".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.device_path,
            Some(PathBuf::from("/dev/bus/usb/001/043"))
        );
        assert_eq!(options.timeout_ms, 250);
        assert_eq!(options.request, "api_length");
    }

    #[test]
    fn parses_usb_run_stats_options() {
        let options = UsbRunStatsOptions::parse(&[
            "--samples".to_string(),
            "12".to_string(),
            "--timeout-ms".to_string(),
            "250".to_string(),
        ])
        .unwrap();

        assert_eq!(options.device_path, None);
        assert_eq!(options.samples, 12);
        assert_eq!(options.timeout_ms, 250);
    }

    #[test]
    fn parses_positional_usb_run_stats_sample_count() {
        let options = UsbRunStatsOptions::parse(&["7".to_string()]).unwrap();

        assert_eq!(options.samples, 7);
    }

    #[test]
    fn formats_usb_run_stats_like_motor_util() {
        let output = format_usb_run_stats_csv(
            "rust",
            UsbRunStats {
                max_fast_loop_cycles: 710,
                max_fast_loop_period: 3416,
                max_main_loop_cycles: 6445,
                max_main_loop_period: 17045,
                mean_fast_loop_cycles_milli: 708_965,
                mean_fast_loop_period_milli: 3_397_560,
                mean_main_loop_cycles_milli: 3_555_490,
                mean_main_loop_period_milli: 16_999_800,
            },
        );

        assert_eq!(
            output,
            "name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period\nrust, 710, 3416, 6445, 17045, 708.965, 3397.56, 3555.49, 16999.8\n"
        );
    }

    #[test]
    fn parses_realtime_usb_options() {
        let options = RealtimeUsbOptions::parse(&[
            "--dev".to_string(),
            "/dev/bus/usb/001/043".to_string(),
            "--sequence".to_string(),
            "77".to_string(),
            "--mode".to_string(),
            "velocity".to_string(),
            "--velocity".to_string(),
            "3.5".to_string(),
            "--timeout-ms".to_string(),
            "250".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.device_path,
            Some(PathBuf::from("/dev/bus/usb/001/043"))
        );
        assert_eq!(options.sequence, 77);
        assert_eq!(options.mode, ControlMode::Velocity);
        assert_eq!(options.velocity_rad_s, 3.5);
        assert_eq!(options.timeout_ms, 250);
    }

    #[test]
    fn realtime_usb_options_can_discover_device_path() {
        let options = RealtimeUsbOptions::parse(&[]).unwrap();

        assert_eq!(options.device_path, None);
        assert_eq!(options.sequence, 1);
        assert_eq!(options.mode, ControlMode::Disabled);
    }

    #[test]
    fn formats_usbfs_path_with_zero_padded_bus_and_device() {
        assert_eq!(
            format_usbfs_path(Path::new("/dev/bus/usb"), 1, 43),
            PathBuf::from("/dev/bus/usb/001/043")
        );
    }

    #[test]
    fn discovers_usb_device_path_from_sysfs_identity() {
        let root = env::temp_dir().join(format!(
            "obot-bench-debug-test-{}-{}",
            std::process::id(),
            monotonic_suffix()
        ));
        let sysfs = root.join("sys");
        let usbfs = root.join("dev");
        let device = sysfs.join("1-10");
        fs::create_dir_all(&device).unwrap();
        fs::create_dir_all(usbfs.join("001")).unwrap();
        fs::write(device.join("idVendor"), "3293\n").unwrap();
        fs::write(device.join("idProduct"), "0100\n").unwrap();
        fs::write(device.join("busnum"), "1\n").unwrap();
        fs::write(device.join("devnum"), "43\n").unwrap();

        let discovered = discover_usb_device_path(
            &sysfs,
            &usbfs,
            usb_control::VENDOR_ID,
            usb_control::PRODUCT_ID,
        )
        .unwrap();

        assert_eq!(discovered, usbfs.join("001").join("043"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_command_write_options() {
        let options = CommandWriteOptions::parse(&[
            "--packet-address".to_string(),
            "0x20000090".to_string(),
            "--sequence-address".to_string(),
            "0x2000009e".to_string(),
            "--sequence".to_string(),
            "7".to_string(),
            "--mode".to_string(),
            "torque".to_string(),
            "--torque".to_string(),
            "1.5".to_string(),
        ])
        .unwrap();

        assert_eq!(options.packet_address, Some(0x2000_0090));
        assert_eq!(options.sequence_address, Some(0x2000_009e));
        assert_eq!(options.sequence, 7);
        assert_eq!(options.mode, ControlMode::Torque);
        assert_eq!(options.torque_nm, 1.5);
    }

    #[test]
    fn parses_clear_faults_command_mode() {
        assert_eq!(
            parse_control_mode("clear-faults").unwrap(),
            ControlMode::ClearFaults
        );
        assert_eq!(
            parse_control_mode("clear_faults").unwrap(),
            ControlMode::ClearFaults
        );
    }

    #[test]
    fn builds_command_write_script_with_packet_before_sequence() {
        let options = ResolvedCommandWriteOptions {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            packet_address: 0x2000_0090,
            sequence_address: 0x2000_009e,
            sequence: 7,
            mode: ControlMode::Torque,
            torque_nm: 1.25,
            velocity_rad_s: 0.0,
            position_rad: 0.0,
        };
        let packet = CommandPacket {
            sequence: options.sequence,
            command: MotorCommand {
                mode: options.mode,
                torque_nm: options.torque_nm,
                velocity_rad_s: options.velocity_rad_s,
                position_rad: options.position_rad,
            },
        };

        let script = jlink_write_command_script(&options, &packet.encode());

        assert!(script.starts_with("device STM32G474RE\nif SWD\nspeed 4000\nconnect\n"));
        assert!(script.contains("w1 0x20000090, 0x07\n"));
        assert!(script.contains("w1 0x20000091, 0x01\n"));
        assert!(script.contains("w1 0x20000094, 0xA0\n"));
        assert!(script.ends_with("w1 0x2000009E, 0x07\ng\nexit\n"));
    }
}
