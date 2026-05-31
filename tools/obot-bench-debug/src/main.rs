use std::{
    env, fs, io,
    path::Path,
    process::{Command, ExitCode},
};

use obot_protocol::{BENCHMARK_PACKET_LEN, BenchmarkPacket};

const DEFAULT_NAME: &str = "rust_debug";
const DEFAULT_DEVICE: &str = "STM32G474RE";
const DEFAULT_ADDRESS: u32 = 0x2000_0000;
const DEFAULT_SPEED_KHZ: u32 = 4_000;

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
        "read-jlink" => read_jlink_command(rest),
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

fn read_jlink_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    let script = jlink_script(&options);
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

    let bytes = parse_jlink_mem8_output(&stdout, BENCHMARK_PACKET_LEN)?;
    decode_packet_csv(&bytes)
}

fn jlink_script_command(args: &[String]) -> Result<String, String> {
    let options = JlinkOptions::parse(args)?;
    Ok(jlink_script(&options))
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

fn parse_u32_arg(value: Option<&String>, flag: &str) -> Result<u32, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a value"))?;
    parse_u32(value).ok_or_else(|| format!("invalid {flag} value `{value}`"))
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

fn jlink_script(options: &JlinkOptions) -> String {
    format!(
        "device {}\nif SWD\nspeed {}\nconnect\nmem8 0x{:08X} {}\nexit\n",
        options.device, options.speed_khz, options.address, BENCHMARK_PACKET_LEN
    )
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
        "usage:\n  obot-bench-debug decode-hex <{} packet bytes as hex>\n  obot-bench-debug decode-file <path-to-raw-{}-byte-packet>\n  obot-bench-debug jlink-script [--address 0x20000000] [--speed 4000]\n  obot-bench-debug read-jlink [--address 0x20000000] [--speed 4000]\n",
        BENCHMARK_PACKET_LEN, BENCHMARK_PACKET_LEN
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
}
