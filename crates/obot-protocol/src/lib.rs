#![no_std]

use obot_core::{
    ControlMode, Fault, MotorCommand, MotorState,
    benchmark::{BenchmarkReport, CycleStatsSnapshot, LoopBenchmarkSnapshot},
    output::OutputSafetyStatus,
};

pub const COMMAND_PACKET_LEN: usize = 14;
pub const DRIVER_COMMAND_PACKET_LEN: usize = 2;
pub const STATUS_PACKET_LEN: usize = 14;
pub const DRIVER_REPORT_PACKET_LEN: usize = 14;
pub const OUTPUT_SAFETY_PACKET_LEN: usize = 2;
pub const BUS_VOLTAGE_PACKET_LEN: usize = 3;
pub const BENCHMARK_PACKET_LEN: usize = 81;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    InvalidLength,
    InvalidMode,
    InvalidFault,
    InvalidDriverCommand,
    InvalidOutputSafetyFlags,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommandPacket {
    pub sequence: u8,
    pub command: MotorCommand,
}

impl CommandPacket {
    pub fn encode(self) -> [u8; COMMAND_PACKET_LEN] {
        let mut out = [0; COMMAND_PACKET_LEN];
        out[0] = self.sequence;
        out[1] = mode_to_u8(self.command.mode);
        out[2..6].copy_from_slice(&self.command.torque_nm.to_le_bytes());
        out[6..10].copy_from_slice(&self.command.velocity_rad_s.to_le_bytes());
        out[10..14].copy_from_slice(&self.command.position_rad.to_le_bytes());
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; COMMAND_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            command: MotorCommand {
                mode: mode_from_u8(bytes[1])?,
                torque_nm: f32::from_le_bytes(bytes[2..6].try_into().unwrap()),
                velocity_rad_s: f32::from_le_bytes(bytes[6..10].try_into().unwrap()),
                position_rad: f32::from_le_bytes(bytes[10..14].try_into().unwrap()),
            },
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverCommand {
    Disable,
    ConfigureEnable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriverCommandPacket {
    pub sequence: u8,
    pub command: DriverCommand,
}

impl DriverCommandPacket {
    pub fn encode(self) -> [u8; DRIVER_COMMAND_PACKET_LEN] {
        [self.sequence, driver_command_to_u8(self.command)]
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; DRIVER_COMMAND_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            command: driver_command_from_u8(bytes[1])?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusPacket {
    pub sequence: u8,
    pub state: MotorState,
}

impl StatusPacket {
    pub fn encode(self) -> [u8; STATUS_PACKET_LEN] {
        let mut out = [0; STATUS_PACKET_LEN];
        out[0] = self.sequence;
        out[1] = fault_to_u8(self.state.fault);
        out[2..6].copy_from_slice(&self.state.torque_nm.to_le_bytes());
        out[6..10].copy_from_slice(&self.state.velocity_rad_s.to_le_bytes());
        out[10..14].copy_from_slice(&self.state.position_rad.to_le_bytes());
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; STATUS_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            state: MotorState {
                fault: fault_from_u8(bytes[1])?,
                torque_nm: f32::from_le_bytes(bytes[2..6].try_into().unwrap()),
                velocity_rad_s: f32::from_le_bytes(bytes[6..10].try_into().unwrap()),
                position_rad: f32::from_le_bytes(bytes[10..14].try_into().unwrap()),
            },
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriverReportPacket {
    pub sequence: u8,
    pub configured: bool,
    pub verify_error_mask: u16,
    pub transfer_error_mask: u16,
    pub status_before: u32,
    pub status_after: u32,
}

impl DriverReportPacket {
    pub fn encode(self) -> [u8; DRIVER_REPORT_PACKET_LEN] {
        let mut out = [0; DRIVER_REPORT_PACKET_LEN];
        out[0] = self.sequence;
        out[1] = u8::from(self.configured);
        out[2..4].copy_from_slice(&self.verify_error_mask.to_le_bytes());
        out[4..6].copy_from_slice(&self.transfer_error_mask.to_le_bytes());
        out[6..10].copy_from_slice(&self.status_before.to_le_bytes());
        out[10..14].copy_from_slice(&self.status_after.to_le_bytes());
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; DRIVER_REPORT_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            configured: bytes[1] != 0,
            verify_error_mask: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            transfer_error_mask: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            status_before: u32::from_le_bytes(bytes[6..10].try_into().unwrap()),
            status_after: u32::from_le_bytes(bytes[10..14].try_into().unwrap()),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputSafetyPacket {
    pub sequence: u8,
    pub status: OutputSafetyStatus,
}

impl OutputSafetyPacket {
    pub fn encode(self) -> [u8; OUTPUT_SAFETY_PACKET_LEN] {
        [self.sequence, output_safety_flags_to_u8(self.status)]
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; OUTPUT_SAFETY_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            status: output_safety_flags_from_u8(bytes[1])?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BusVoltagePacket {
    pub sequence: u8,
    pub raw: u16,
}

impl BusVoltagePacket {
    pub fn encode(self) -> [u8; BUS_VOLTAGE_PACKET_LEN] {
        let mut out = [0; BUS_VOLTAGE_PACKET_LEN];
        out[0] = self.sequence;
        out[1..3].copy_from_slice(&self.raw.to_le_bytes());
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; BUS_VOLTAGE_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;

        Ok(Self {
            sequence: bytes[0],
            raw: u16::from_le_bytes(bytes[1..3].try_into().unwrap()),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchmarkPacket {
    pub sequence: u8,
    pub report: BenchmarkReport,
}

impl BenchmarkPacket {
    pub fn encode(self) -> [u8; BENCHMARK_PACKET_LEN] {
        let mut out = [0; BENCHMARK_PACKET_LEN];
        out[0] = self.sequence;
        let mut offset = 1;
        offset = encode_loop_snapshot(&mut out, offset, self.report.fast);
        encode_loop_snapshot(&mut out, offset, self.report.main);
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let bytes: &[u8; BENCHMARK_PACKET_LEN] =
            input.try_into().map_err(|_| DecodeError::InvalidLength)?;
        let mut offset = 1;
        let (fast, next_offset) = decode_loop_snapshot(bytes, offset);
        offset = next_offset;
        let (main, _) = decode_loop_snapshot(bytes, offset);

        Ok(Self {
            sequence: bytes[0],
            report: BenchmarkReport { fast, main },
        })
    }
}

fn encode_loop_snapshot(
    out: &mut [u8; BENCHMARK_PACKET_LEN],
    offset: usize,
    snapshot: LoopBenchmarkSnapshot,
) -> usize {
    let offset = encode_stats_snapshot(out, offset, snapshot.period);
    encode_stats_snapshot(out, offset, snapshot.execution)
}

fn encode_stats_snapshot(
    out: &mut [u8; BENCHMARK_PACKET_LEN],
    offset: usize,
    snapshot: CycleStatsSnapshot,
) -> usize {
    out[offset..offset + 4].copy_from_slice(&snapshot.samples.to_le_bytes());
    out[offset + 4..offset + 8].copy_from_slice(&snapshot.last_cycles.to_le_bytes());
    out[offset + 8..offset + 12].copy_from_slice(&snapshot.max_cycles.to_le_bytes());
    out[offset + 12..offset + 20].copy_from_slice(&snapshot.mean_milli_cycles.to_le_bytes());
    offset + 20
}

fn decode_loop_snapshot(
    input: &[u8; BENCHMARK_PACKET_LEN],
    offset: usize,
) -> (LoopBenchmarkSnapshot, usize) {
    let (period, offset) = decode_stats_snapshot(input, offset);
    let (execution, offset) = decode_stats_snapshot(input, offset);
    (LoopBenchmarkSnapshot { period, execution }, offset)
}

fn decode_stats_snapshot(
    input: &[u8; BENCHMARK_PACKET_LEN],
    offset: usize,
) -> (CycleStatsSnapshot, usize) {
    let samples = u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap());
    let last_cycles = u32::from_le_bytes(input[offset + 4..offset + 8].try_into().unwrap());
    let max_cycles = u32::from_le_bytes(input[offset + 8..offset + 12].try_into().unwrap());
    let mean_milli_cycles = u64::from_le_bytes(input[offset + 12..offset + 20].try_into().unwrap());

    (
        CycleStatsSnapshot {
            samples,
            last_cycles,
            max_cycles,
            mean_milli_cycles,
        },
        offset + 20,
    )
}

const fn mode_to_u8(mode: ControlMode) -> u8 {
    match mode {
        ControlMode::Disabled => 0,
        ControlMode::Torque => 1,
        ControlMode::Velocity => 2,
        ControlMode::Position => 3,
        ControlMode::ClearFaults => 250,
    }
}

const fn mode_from_u8(value: u8) -> Result<ControlMode, DecodeError> {
    match value {
        0 => Ok(ControlMode::Disabled),
        1 => Ok(ControlMode::Torque),
        2 => Ok(ControlMode::Velocity),
        3 => Ok(ControlMode::Position),
        250 => Ok(ControlMode::ClearFaults),
        _ => Err(DecodeError::InvalidMode),
    }
}

const fn fault_to_u8(fault: Option<Fault>) -> u8 {
    match fault {
        None => 0,
        Some(Fault::CommandNotFinite) => 1,
        Some(Fault::TorqueLimit) => 2,
        Some(Fault::VelocityLimit) => 3,
        Some(Fault::PositionLimit) => 4,
    }
}

const fn fault_from_u8(value: u8) -> Result<Option<Fault>, DecodeError> {
    match value {
        0 => Ok(None),
        1 => Ok(Some(Fault::CommandNotFinite)),
        2 => Ok(Some(Fault::TorqueLimit)),
        3 => Ok(Some(Fault::VelocityLimit)),
        4 => Ok(Some(Fault::PositionLimit)),
        _ => Err(DecodeError::InvalidFault),
    }
}

const OUTPUT_SAFETY_KNOWN_FLAGS: u8 = 0x7f;

const fn output_safety_flags_to_u8(status: OutputSafetyStatus) -> u8 {
    bool_to_u8(status.output_allowed)
        | (bool_to_u8(status.command_blocked) << 1)
        | (bool_to_u8(status.bus_blocked) << 2)
        | (bool_to_u8(status.driver_not_enabled) << 3)
        | (bool_to_u8(status.driver_fault_latched) << 4)
        | (bool_to_u8(status.controller_faulted) << 5)
        | (bool_to_u8(status.host_timed_out) << 6)
}

const fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

const fn output_safety_flags_from_u8(value: u8) -> Result<OutputSafetyStatus, DecodeError> {
    if value & !OUTPUT_SAFETY_KNOWN_FLAGS != 0 {
        return Err(DecodeError::InvalidOutputSafetyFlags);
    }

    Ok(OutputSafetyStatus {
        output_allowed: value & (1 << 0) != 0,
        command_blocked: value & (1 << 1) != 0,
        bus_blocked: value & (1 << 2) != 0,
        driver_not_enabled: value & (1 << 3) != 0,
        driver_fault_latched: value & (1 << 4) != 0,
        controller_faulted: value & (1 << 5) != 0,
        host_timed_out: value & (1 << 6) != 0,
    })
}

const fn driver_command_to_u8(command: DriverCommand) -> u8 {
    match command {
        DriverCommand::Disable => 0,
        DriverCommand::ConfigureEnable => 1,
    }
}

const fn driver_command_from_u8(value: u8) -> Result<DriverCommand, DecodeError> {
    match value {
        0 => Ok(DriverCommand::Disable),
        1 => Ok(DriverCommand::ConfigureEnable),
        _ => Err(DecodeError::InvalidDriverCommand),
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_packet_round_trips() {
        let packet = CommandPacket {
            sequence: 7,
            command: MotorCommand {
                torque_nm: 1.0,
                velocity_rad_s: 2.0,
                position_rad: 3.0,
                mode: ControlMode::Position,
            },
        };

        assert_eq!(CommandPacket::decode(&packet.encode()).unwrap(), packet);
    }

    #[test]
    fn clear_faults_mode_matches_cpp_value() {
        let packet = CommandPacket {
            sequence: 1,
            command: MotorCommand {
                mode: ControlMode::ClearFaults,
                ..MotorCommand::default()
            },
        };
        let encoded = packet.encode();

        assert_eq!(encoded[1], 250);
        assert_eq!(CommandPacket::decode(&encoded).unwrap(), packet);
    }

    #[test]
    fn driver_command_packet_round_trips() {
        let packet = DriverCommandPacket {
            sequence: 11,
            command: DriverCommand::ConfigureEnable,
        };

        assert_eq!(
            DriverCommandPacket::decode(&packet.encode()).unwrap(),
            packet
        );
    }

    #[test]
    fn status_packet_round_trips() {
        let packet = StatusPacket {
            sequence: 8,
            state: MotorState {
                torque_nm: 1.0,
                velocity_rad_s: 2.0,
                position_rad: 3.0,
                fault: Some(Fault::VelocityLimit),
            },
        };

        assert_eq!(StatusPacket::decode(&packet.encode()).unwrap(), packet);
    }

    #[test]
    fn driver_report_packet_round_trips() {
        let packet = DriverReportPacket {
            sequence: 3,
            configured: true,
            verify_error_mask: 0x0012,
            transfer_error_mask: 0x0040,
            status_before: 0xAABB_CCDD,
            status_after: 0x1122_3344,
        };

        assert_eq!(
            DriverReportPacket::decode(&packet.encode()).unwrap(),
            packet
        );
    }

    #[test]
    fn output_safety_packet_round_trips() {
        let packet = OutputSafetyPacket {
            sequence: 12,
            status: OutputSafetyStatus {
                output_allowed: true,
                command_blocked: false,
                bus_blocked: true,
                driver_not_enabled: true,
                driver_fault_latched: false,
                controller_faulted: true,
                host_timed_out: true,
            },
        };
        let encoded = packet.encode();

        assert_eq!(encoded, [12, 0b0110_1101]);
        assert_eq!(OutputSafetyPacket::decode(&encoded).unwrap(), packet);
    }

    #[test]
    fn bus_voltage_packet_round_trips() {
        let packet = BusVoltagePacket {
            sequence: 4,
            raw: 1_963,
        };

        assert_eq!(packet.encode(), [4, 0xAB, 0x07]);
        assert_eq!(BusVoltagePacket::decode(&packet.encode()).unwrap(), packet);
    }

    #[test]
    fn rejects_unknown_output_safety_flags() {
        assert_eq!(
            OutputSafetyPacket::decode(&[0, 0x80]).unwrap_err(),
            DecodeError::InvalidOutputSafetyFlags
        );
    }

    #[test]
    fn rejects_unknown_mode() {
        let mut bytes = [0; COMMAND_PACKET_LEN];
        bytes[1] = 99;

        assert_eq!(
            CommandPacket::decode(&bytes).unwrap_err(),
            DecodeError::InvalidMode
        );
    }

    #[test]
    fn rejects_unknown_driver_command() {
        let bytes = [0, 99];

        assert_eq!(
            DriverCommandPacket::decode(&bytes).unwrap_err(),
            DecodeError::InvalidDriverCommand
        );
    }

    #[test]
    fn benchmark_packet_round_trips() {
        let packet = BenchmarkPacket {
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
        };

        assert_eq!(BenchmarkPacket::decode(&packet.encode()).unwrap(), packet);
    }
}
