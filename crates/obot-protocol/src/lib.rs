#![no_std]

use obot_core::{ControlMode, Fault, MotorCommand, MotorState};

pub const COMMAND_PACKET_LEN: usize = 14;
pub const STATUS_PACKET_LEN: usize = 14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    InvalidLength,
    InvalidMode,
    InvalidFault,
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

const fn mode_to_u8(mode: ControlMode) -> u8 {
    match mode {
        ControlMode::Disabled => 0,
        ControlMode::Torque => 1,
        ControlMode::Velocity => 2,
        ControlMode::Position => 3,
    }
}

const fn mode_from_u8(value: u8) -> Result<ControlMode, DecodeError> {
    match value {
        0 => Ok(ControlMode::Disabled),
        1 => Ok(ControlMode::Torque),
        2 => Ok(ControlMode::Velocity),
        3 => Ok(ControlMode::Position),
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
    fn rejects_unknown_mode() {
        let mut bytes = [0; COMMAND_PACKET_LEN];
        bytes[1] = 99;

        assert_eq!(
            CommandPacket::decode(&bytes).unwrap_err(),
            DecodeError::InvalidMode
        );
    }
}
