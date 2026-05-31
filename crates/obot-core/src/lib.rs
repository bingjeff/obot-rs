#![no_std]

pub mod benchmark;
pub mod hall;
pub mod timing;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorCommand {
    pub torque_nm: f32,
    pub velocity_rad_s: f32,
    pub position_rad: f32,
    pub mode: ControlMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ControlMode {
    #[default]
    Disabled,
    Torque,
    Velocity,
    Position,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorState {
    pub torque_nm: f32,
    pub velocity_rad_s: f32,
    pub position_rad: f32,
    pub fault: Option<Fault>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    CommandNotFinite,
    TorqueLimit,
    VelocityLimit,
    PositionLimit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_torque_nm: f32,
    pub max_velocity_rad_s: f32,
    pub min_position_rad: f32,
    pub max_position_rad: f32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_torque_nm: 0.0,
            max_velocity_rad_s: 0.0,
            min_position_rad: 0.0,
            max_position_rad: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controller {
    limits: Limits,
    state: MotorState,
}

impl Controller {
    pub const fn new(limits: Limits) -> Self {
        Self {
            limits,
            state: MotorState {
                torque_nm: 0.0,
                velocity_rad_s: 0.0,
                position_rad: 0.0,
                fault: None,
            },
        }
    }

    pub const fn state(&self) -> MotorState {
        self.state
    }

    pub fn apply(&mut self, command: MotorCommand) -> Result<MotorState, Fault> {
        if !command.torque_nm.is_finite()
            || !command.velocity_rad_s.is_finite()
            || !command.position_rad.is_finite()
        {
            return self.latch_fault(Fault::CommandNotFinite);
        }

        if command.torque_nm.abs() > self.limits.max_torque_nm {
            return self.latch_fault(Fault::TorqueLimit);
        }

        if command.velocity_rad_s.abs() > self.limits.max_velocity_rad_s {
            return self.latch_fault(Fault::VelocityLimit);
        }

        if command.position_rad < self.limits.min_position_rad
            || command.position_rad > self.limits.max_position_rad
        {
            return self.latch_fault(Fault::PositionLimit);
        }

        self.state = match command.mode {
            ControlMode::Disabled => MotorState::default(),
            ControlMode::Torque => MotorState {
                torque_nm: command.torque_nm,
                ..MotorState::default()
            },
            ControlMode::Velocity => MotorState {
                velocity_rad_s: command.velocity_rad_s,
                ..MotorState::default()
            },
            ControlMode::Position => MotorState {
                position_rad: command.position_rad,
                ..MotorState::default()
            },
        };

        Ok(self.state)
    }

    pub fn clear_fault(&mut self) {
        self.state.fault = None;
    }

    fn latch_fault(&mut self, fault: Fault) -> Result<MotorState, Fault> {
        self.state.fault = Some(fault);
        Err(fault)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        max_torque_nm: 2.0,
        max_velocity_rad_s: 50.0,
        min_position_rad: -3.0,
        max_position_rad: 3.0,
    };

    #[test]
    fn applies_valid_torque_command() {
        let mut controller = Controller::new(LIMITS);
        let state = controller
            .apply(MotorCommand {
                torque_nm: 1.25,
                mode: ControlMode::Torque,
                ..MotorCommand::default()
            })
            .unwrap();

        assert_eq!(state.torque_nm, 1.25);
        assert_eq!(state.fault, None);
    }

    #[test]
    fn latches_limit_fault() {
        let mut controller = Controller::new(LIMITS);
        let fault = controller
            .apply(MotorCommand {
                torque_nm: 2.5,
                mode: ControlMode::Torque,
                ..MotorCommand::default()
            })
            .unwrap_err();

        assert_eq!(fault, Fault::TorqueLimit);
        assert_eq!(controller.state().fault, Some(Fault::TorqueLimit));
    }
}
