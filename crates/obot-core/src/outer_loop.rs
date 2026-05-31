use crate::{
    ControlMode, MotorState,
    foc::{FirstOrderLowPassFilter, FocDesired, fsat},
    hall::HallMotionEstimate,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PidParam {
    pub kp: f32,
    pub ki: f32,
    pub ki_limit: f32,
    pub kd: f32,
    pub command_max: f32,
    pub velocity_filter_frequency_hz: f32,
    pub output_filter_frequency_hz: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityControllerParam {
    pub velocity: PidParam,
    pub acceleration_limit: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionControllerParam {
    pub position: PidParam,
    pub velocity_limit: f32,
    pub desired_filter_hz: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorHallOuterLoopParam {
    pub iq_per_torque_nm: f32,
    pub max_iq_a: f32,
    pub velocity: VelocityControllerParam,
    pub position: PositionControllerParam,
}

impl MotorHallOuterLoopParam {
    pub const MOTOR_HALL: Self = Self {
        iq_per_torque_nm: 1.0,
        max_iq_a: 1.0,
        velocity: VelocityControllerParam {
            velocity: PidParam {
                kp: 0.01,
                ki: 1.0,
                ki_limit: 0.4,
                kd: 0.0,
                command_max: 0.5,
                velocity_filter_frequency_hz: 0.0,
                output_filter_frequency_hz: 100.0,
            },
            acceleration_limit: 1000.0,
        },
        position: PositionControllerParam {
            position: PidParam {
                kp: 1.0,
                ki: 0.0,
                ki_limit: 0.0,
                kd: 0.01,
                command_max: 0.7,
                velocity_filter_frequency_hz: 1000.0,
                output_filter_frequency_hz: 0.0,
            },
            velocity_limit: 0.0,
            desired_filter_hz: 0.0,
        },
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RateLimiter {
    limit: f32,
    velocity: f32,
    last_value: f32,
}

impl RateLimiter {
    pub const fn new() -> Self {
        Self {
            limit: f32::INFINITY,
            velocity: 0.0,
            last_value: 0.0,
        }
    }

    #[inline(always)]
    pub fn set_limit(&mut self, limit: f32) {
        self.limit = if limit == 0.0 { f32::INFINITY } else { limit };
    }

    #[inline(always)]
    pub fn init(&mut self, value: f32, velocity: f32) {
        self.last_value = value;
        self.velocity = velocity;
    }

    #[inline(always)]
    pub fn step(&mut self, value: f32) -> f32 {
        let out_value = if value > self.last_value + self.limit {
            self.velocity = self.limit;
            self.last_value + self.limit
        } else if value < self.last_value - self.limit {
            self.velocity = -self.limit;
            self.last_value - self.limit
        } else {
            self.velocity = value - self.last_value;
            value
        };
        self.last_value = out_value;
        out_value
    }

    pub const fn value(self) -> f32 {
        self.last_value
    }

    pub const fn velocity(self) -> f32 {
        self.velocity
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SecondOrderLowPassFilter {
    low_pass_1: FirstOrderLowPassFilter,
    low_pass_2: FirstOrderLowPassFilter,
}

impl SecondOrderLowPassFilter {
    pub fn new(dt: f32, frequency_hz: f32) -> Self {
        Self {
            low_pass_1: FirstOrderLowPassFilter::new(dt, frequency_hz),
            low_pass_2: FirstOrderLowPassFilter::new(dt, frequency_hz),
        }
    }

    #[inline(always)]
    pub fn init(&mut self, value: f32) {
        self.low_pass_1.init(value);
        self.low_pass_2.init(value);
    }

    #[inline(always)]
    pub fn update(&mut self, value: f32) -> f32 {
        self.low_pass_2.update(self.low_pass_1.update(value))
    }

    #[inline(always)]
    pub fn set_frequency(&mut self, frequency_hz: f32) {
        self.low_pass_1.set_frequency(frequency_hz);
        self.low_pass_2.set_frequency(frequency_hz);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PidController {
    dt: f32,
    param: PidParam,
    rate_limit: RateLimiter,
    velocity_filter: SecondOrderLowPassFilter,
    output_filter: FirstOrderLowPassFilter,
    ki_sum: f32,
    measured_last: f32,
}

impl PidController {
    pub fn new(dt: f32, param: PidParam) -> Self {
        Self {
            dt,
            param,
            rate_limit: RateLimiter::new(),
            velocity_filter: SecondOrderLowPassFilter::new(dt, param.velocity_filter_frequency_hz),
            output_filter: FirstOrderLowPassFilter::new(dt, param.output_filter_frequency_hz),
            ki_sum: 0.0,
            measured_last: 0.0,
        }
    }

    pub fn set_param(&mut self, param: PidParam) {
        self.param = param;
        self.velocity_filter
            .set_frequency(param.velocity_filter_frequency_hz);
        self.output_filter
            .set_frequency(param.output_filter_frequency_hz);
    }

    pub fn init(&mut self, measured: f32) {
        self.rate_limit.init(measured, 0.0);
        self.ki_sum = 0.0;
        self.measured_last = measured;
        self.velocity_filter.init(0.0);
        self.output_filter.init(0.0);
    }

    #[inline(always)]
    pub fn step(
        &mut self,
        desired: f32,
        velocity_desired: f32,
        measured: f32,
        velocity_limit: f32,
    ) -> f32 {
        self.rate_limit.set_limit((velocity_limit * self.dt).abs());
        let proxy_desired = self.rate_limit.step(desired);
        let proxy_velocity_desired = fsat(velocity_desired, velocity_limit);
        let error = proxy_desired - measured;
        let velocity_measured = self
            .velocity_filter
            .update((measured - self.measured_last) / self.dt);
        let error_dot = proxy_velocity_desired - velocity_measured;
        self.measured_last = measured;
        self.ki_sum = fsat(
            self.ki_sum + self.param.ki * self.dt * error,
            self.param.ki_limit,
        );
        let output = self
            .output_filter
            .update(self.param.kp * error + self.ki_sum + self.param.kd * error_dot);
        fsat(output, self.param.command_max)
    }

    pub const fn ki_sum(self) -> f32 {
        self.ki_sum
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelocityController {
    dt: f32,
    controller: PidController,
    velocity_filter: FirstOrderLowPassFilter,
    acceleration_limit: f32,
    last_motor_position: f32,
}

impl VelocityController {
    pub fn new(dt: f32, param: VelocityControllerParam) -> Self {
        Self {
            dt,
            controller: PidController::new(dt, param.velocity),
            velocity_filter: FirstOrderLowPassFilter::new(
                dt,
                param.velocity.velocity_filter_frequency_hz,
            ),
            acceleration_limit: param.acceleration_limit,
            last_motor_position: 0.0,
        }
    }

    pub fn set_param(&mut self, param: VelocityControllerParam) {
        self.controller.set_param(param.velocity);
        self.velocity_filter
            .set_frequency(param.velocity.velocity_filter_frequency_hz);
        self.acceleration_limit = param.acceleration_limit;
    }

    pub fn init(&mut self, feedback: HallMotionEstimate) {
        self.controller.init(0.0);
        self.velocity_filter.init(0.0);
        self.last_motor_position = feedback.position_rad;
    }

    #[inline(always)]
    pub fn step(&mut self, desired_velocity_rad_s: f32, feedback: HallMotionEstimate) -> f32 {
        let measured_velocity = self
            .velocity_filter
            .update((feedback.position_rad - self.last_motor_position) / self.dt);
        self.last_motor_position = feedback.position_rad;
        self.controller.step(
            desired_velocity_rad_s,
            0.0,
            measured_velocity,
            self.acceleration_limit,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionController {
    controller: PidController,
    desired_filter: SecondOrderLowPassFilter,
    velocity_limit: f32,
}

impl PositionController {
    pub fn new(dt: f32, param: PositionControllerParam) -> Self {
        Self {
            controller: PidController::new(dt, param.position),
            desired_filter: SecondOrderLowPassFilter::new(dt, param.desired_filter_hz),
            velocity_limit: param.velocity_limit,
        }
    }

    pub fn set_param(&mut self, param: PositionControllerParam) {
        self.controller.set_param(param.position);
        self.desired_filter.set_frequency(param.desired_filter_hz);
        self.velocity_limit = param.velocity_limit;
    }

    pub fn init(&mut self, feedback: HallMotionEstimate) {
        self.controller.init(feedback.position_rad);
        self.desired_filter.init(feedback.position_rad);
    }

    #[inline(always)]
    pub fn step(
        &mut self,
        desired_position_rad: f32,
        desired_velocity_rad_s: f32,
        feedback: HallMotionEstimate,
    ) -> f32 {
        let desired_position = self.desired_filter.update(desired_position_rad);
        self.controller.step(
            desired_position,
            desired_velocity_rad_s,
            feedback.position_rad,
            self.velocity_limit,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorHallOuterLoop {
    param: MotorHallOuterLoopParam,
    velocity: VelocityController,
    position: PositionController,
    last_mode: ControlMode,
    initialized: bool,
}

impl MotorHallOuterLoop {
    pub fn new(param: MotorHallOuterLoopParam, dt: f32) -> Self {
        Self {
            param,
            velocity: VelocityController::new(dt, param.velocity),
            position: PositionController::new(dt, param.position),
            last_mode: ControlMode::Disabled,
            initialized: false,
        }
    }

    pub fn set_param(&mut self, param: MotorHallOuterLoopParam) {
        self.param = param;
        self.velocity.set_param(param.velocity);
        self.position.set_param(param.position);
    }

    #[inline(always)]
    pub fn desired_from_state(
        &mut self,
        state: MotorState,
        feedback: HallMotionEstimate,
    ) -> FocDesired {
        if state.fault.is_some()
            || matches!(state.mode, ControlMode::Disabled | ControlMode::ClearFaults)
        {
            self.last_mode = ControlMode::Disabled;
            self.initialized = false;
            return FocDesired::default();
        }

        if !self.initialized || state.mode != self.last_mode {
            self.init_mode(state.mode, feedback);
        }

        match state.mode {
            ControlMode::Torque => FocDesired {
                i_q: fsat(
                    state.torque_nm * self.param.iq_per_torque_nm,
                    self.param.max_iq_a,
                ),
                ..FocDesired::default()
            },
            ControlMode::Velocity => FocDesired {
                i_q: self.velocity.step(state.velocity_rad_s, feedback),
                ..FocDesired::default()
            },
            ControlMode::Position => FocDesired {
                i_q: self
                    .position
                    .step(state.position_rad, state.velocity_rad_s, feedback),
                ..FocDesired::default()
            },
            ControlMode::Disabled | ControlMode::ClearFaults => FocDesired::default(),
        }
    }

    fn init_mode(&mut self, mode: ControlMode, feedback: HallMotionEstimate) {
        self.velocity.init(feedback);
        self.position.init(feedback);
        self.last_mode = mode;
        self.initialized = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-5;
    const DT_10_KHZ: f32 = 1.0 / 10_000.0;

    fn feedback(position_rad: f32) -> HallMotionEstimate {
        HallMotionEstimate {
            position_rad,
            ..HallMotionEstimate::default()
        }
    }

    #[test]
    fn motor_hall_outer_loop_keeps_disabled_and_faulted_commands_zero() {
        let mut controller =
            MotorHallOuterLoop::new(MotorHallOuterLoopParam::MOTOR_HALL, DT_10_KHZ);

        assert_eq!(
            controller.desired_from_state(MotorState::default(), feedback(0.0)),
            FocDesired::default()
        );
        assert_eq!(
            controller.desired_from_state(
                MotorState {
                    mode: ControlMode::Torque,
                    torque_nm: 0.25,
                    fault: Some(crate::Fault::TorqueLimit),
                    ..MotorState::default()
                },
                feedback(0.0)
            ),
            FocDesired::default()
        );
    }

    #[test]
    fn torque_mode_maps_to_limited_q_axis_current() {
        let mut controller =
            MotorHallOuterLoop::new(MotorHallOuterLoopParam::MOTOR_HALL, DT_10_KHZ);

        assert_close(
            controller
                .desired_from_state(
                    MotorState {
                        mode: ControlMode::Torque,
                        torque_nm: 2.0,
                        ..MotorState::default()
                    },
                    feedback(0.0),
                )
                .i_q,
            1.0,
        );
    }

    #[test]
    fn velocity_mode_uses_motor_hall_rate_limited_pi() {
        let mut controller =
            MotorHallOuterLoop::new(MotorHallOuterLoopParam::MOTOR_HALL, DT_10_KHZ);
        let output = controller.desired_from_state(
            MotorState {
                mode: ControlMode::Velocity,
                velocity_rad_s: 10.0,
                ..MotorState::default()
            },
            feedback(0.0),
        );
        let alpha = 2.0 * core::f32::consts::PI * DT_10_KHZ * 100.0
            / (2.0 * core::f32::consts::PI * DT_10_KHZ * 100.0 + 1.0);
        let rate_limited_velocity = 1000.0 * DT_10_KHZ;
        let expected_unfiltered =
            0.01 * rate_limited_velocity + 1.0 * DT_10_KHZ * rate_limited_velocity;

        assert_close(output.i_q, alpha * expected_unfiltered);
    }

    #[test]
    fn position_mode_uses_motor_hall_pd_limit() {
        let mut controller =
            MotorHallOuterLoop::new(MotorHallOuterLoopParam::MOTOR_HALL, DT_10_KHZ);
        let output = controller.desired_from_state(
            MotorState {
                mode: ControlMode::Position,
                position_rad: 1.0,
                ..MotorState::default()
            },
            feedback(0.0),
        );

        assert_close(output.i_q, 0.7);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "actual={actual}, expected={expected}"
        );
    }
}
