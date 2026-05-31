use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use obot_core::{ControlMode, MotorCommand};
use obot_protocol::{
    BENCHMARK_PACKET_LEN, BenchmarkPacket, CommandPacket, DRIVER_REPORT_PACKET_LEN, DriverCommand,
    DriverCommandPacket, DriverReportPacket, OUTPUT_SAFETY_PACKET_LEN, OutputSafetyPacket,
    STATUS_PACKET_LEN, StatusPacket,
};

const DEFAULT_NAME: &str = "rust_debug";
const DEFAULT_DEVICE: &str = "STM32G474RE";
const DEFAULT_ADDRESS: u32 = 0x2000_0000;
const DEFAULT_SPEED_KHZ: u32 = 4_000;
const DEFAULT_ELF_PATH: &str = "target/thumbv7em-none-eabihf/release/obot-g474";
const BENCHMARK_PACKET_SYMBOL: &str = "OBOT_BENCHMARK_PACKET";
const OUTPUT_SAFETY_PACKET_SYMBOL: &str = "OBOT_OUTPUT_SAFETY_PACKET";

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
        "read-jlink" => read_jlink_command(rest),
        "read-jlink-detail" => read_jlink_detail_command(rest),
        "run-stats-jlink" => run_stats_jlink_command(rest),
        "read-status-jlink" => read_status_jlink_command(rest),
        "read-driver-jlink" => read_driver_jlink_command(rest),
        "read-output-safety-jlink" => read_output_safety_jlink_command(rest),
        "write-command-jlink" => write_command_jlink_command(rest),
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

fn read_status_jlink_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    let bytes = read_jlink_bytes(&options, STATUS_PACKET_LEN)?;
    decode_status_csv(&bytes)
}

fn read_driver_jlink_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    let bytes = read_jlink_bytes(&options, DRIVER_REPORT_PACKET_LEN)?;
    decode_driver_csv(&bytes)
}

fn read_output_safety_jlink_command(args: &[String]) -> Result<String, String> {
    let options = SymbolReadOptions::parse(args)?;
    let jlink = options.resolve(OUTPUT_SAFETY_PACKET_SYMBOL)?;
    let bytes = read_jlink_bytes(&jlink, OUTPUT_SAFETY_PACKET_LEN)?;
    decode_output_safety_csv(&bytes)
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
    let options = CommandWriteOptions::parse(args)?;
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

fn write_driver_command_jlink_command(args: &[String]) -> Result<String, String> {
    let options = DriverCommandWriteOptions::parse(args)?;
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct CommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    mode: ControlMode,
    torque_nm: f32,
    velocity_rad_s: f32,
    position_rad: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DriverCommandWriteOptions {
    jlink: JlinkOptions,
    packet_address: u32,
    sequence_address: u32,
    sequence: u8,
    command: DriverCommand,
}

impl DriverCommandWriteOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            jlink: JlinkOptions {
                address: DEFAULT_ADDRESS,
                speed_khz: DEFAULT_SPEED_KHZ,
                device: DEFAULT_DEVICE,
            },
            packet_address: 0,
            sequence_address: 0,
            sequence: 1,
            command: DriverCommand::Disable,
        };
        let mut packet_address_set = false;
        let mut sequence_address_set = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--packet-address" => {
                    index += 1;
                    options.packet_address = parse_u32_arg(args.get(index), "--packet-address")?;
                    packet_address_set = true;
                }
                "--sequence-address" => {
                    index += 1;
                    options.sequence_address =
                        parse_u32_arg(args.get(index), "--sequence-address")?;
                    sequence_address_set = true;
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

        if !packet_address_set {
            return Err("write-driver-command-jlink requires --packet-address".to_string());
        }
        if !sequence_address_set {
            return Err("write-driver-command-jlink requires --sequence-address".to_string());
        }

        Ok(options)
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
            packet_address: 0,
            sequence_address: 0,
            sequence: 1,
            mode: ControlMode::Disabled,
            torque_nm: 0.0,
            velocity_rad_s: 0.0,
            position_rad: 0.0,
        };
        let mut packet_address_set = false;
        let mut sequence_address_set = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--packet-address" => {
                    index += 1;
                    options.packet_address = parse_u32_arg(args.get(index), "--packet-address")?;
                    packet_address_set = true;
                }
                "--sequence-address" => {
                    index += 1;
                    options.sequence_address =
                        parse_u32_arg(args.get(index), "--sequence-address")?;
                    sequence_address_set = true;
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

        if !packet_address_set {
            return Err("write-command-jlink requires --packet-address".to_string());
        }
        if !sequence_address_set {
            return Err("write-command-jlink requires --sequence-address".to_string());
        }

        Ok(options)
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
        let entry = entry.map_err(|error| {
            format!("failed to inspect `{}` entry: {error}", rustlib.display())
        })?;
        let candidate = entry.path().join("bin").join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!("could not find `{name}` under `{}`", rustlib.display()))
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
        "device {}\nif SWD\nspeed {}\nconnect\nmem8 0x{:08X} {}\nexit\n",
        options.device, options.speed_khz, options.address, len
    )
}

fn jlink_write_command_script(options: &CommandWriteOptions, encoded_packet: &[u8]) -> String {
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
        "w1 0x{:08X}, 0x{:02X}\nexit\n",
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
        "name, sequence, output_allowed, command_blocked, bus_blocked, driver_not_enabled, driver_fault_latched, controller_faulted\n{}, {}, {}, {}, {}, {}, {}, {}\n",
        name,
        packet.sequence,
        status.output_allowed,
        status.command_blocked,
        status.bus_blocked,
        status.driver_not_enabled,
        status.driver_fault_latched,
        status.controller_faulted,
    )
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
  obot-bench-debug jlink-script [--address 0x20000000] [--speed 4000]
  obot-bench-debug read-jlink [--address 0x20000000] [--speed 4000]
  obot-bench-debug read-jlink-detail [--address 0x20000000] [--speed 4000]
  obot-bench-debug run-stats-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address 0x20000000] [--speed 4000]
  obot-bench-debug read-status-jlink --address <status-packet-address> [--speed 4000]
  obot-bench-debug read-driver-jlink --address <driver-report-address> [--speed 4000]
  obot-bench-debug read-output-safety-jlink [--elf target/thumbv7em-none-eabihf/release/obot-g474] [--address <output-safety-address>] [--speed 4000]
  obot-bench-debug write-command-jlink --packet-address <command-packet-address> --sequence-address <command-sequence-address> [--sequence N] [--mode disabled|torque|velocity|position|clear-faults] [--torque Nm] [--velocity rad_s] [--position rad]
  obot-bench-debug write-driver-command-jlink --packet-address <driver-command-packet-address> --sequence-address <driver-command-sequence-address> [--sequence N] [--command disable|configure-enable]
",
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        BENCHMARK_PACKET_LEN,
        STATUS_PACKET_LEN,
        DRIVER_REPORT_PACKET_LEN,
        OUTPUT_SAFETY_PACKET_LEN
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
                },
            },
        );

        assert_eq!(
            output,
            "name, sequence, output_allowed, command_blocked, bus_blocked, driver_not_enabled, driver_fault_latched, controller_faulted\nrust, 5, false, true, true, true, false, true\n"
        );
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

        assert_eq!(options.packet_address, 0x2000_0090);
        assert_eq!(options.sequence_address, 0x2000_0092);
        assert_eq!(options.sequence, 8);
        assert_eq!(options.command, DriverCommand::ConfigureEnable);
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

        assert_eq!(options.packet_address, 0x2000_0090);
        assert_eq!(options.sequence_address, 0x2000_009e);
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
        let options = CommandWriteOptions {
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
        assert!(script.ends_with("w1 0x2000009E, 0x07\nexit\n"));
    }
}
