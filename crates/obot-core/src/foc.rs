use crate::current::PhaseCurrents;

const TWO_THIRDS: f32 = 2.0 / 3.0;
const NEG_ONE_THIRD: f32 = -1.0 / 3.0;
const ONE_OVER_SQRT3: f32 = 0.577_350_26;
const TWO_PI: f32 = 2.0 * core::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiParam {
    pub kp: f32,
    pub ki: f32,
    pub ki_limit: f32,
    pub command_max: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocParam {
    pub pi_d: PiParam,
    pub pi_q: PiParam,
    pub current_filter_frequency_hz: f32,
    pub num_poles: f32,
}

impl FocParam {
    pub const MOTOR_HALL: Self = Self {
        pi_d: PiParam {
            kp: 7.0,
            ki: 0.5,
            ki_limit: 8.0,
            command_max: 10.0,
        },
        pi_q: PiParam {
            kp: 7.0,
            ki: 0.5,
            ki_limit: 8.0,
            command_max: 10.0,
        },
        current_filter_frequency_hz: 20_000.0,
        num_poles: 7.0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiController {
    param: PiParam,
    ki_sum: f32,
}

impl PiController {
    pub const fn new(param: PiParam) -> Self {
        Self { param, ki_sum: 0.0 }
    }

    #[inline(always)]
    pub fn initialize(&mut self) {
        self.ki_sum = 0.0;
    }

    #[inline(always)]
    pub fn set_param(&mut self, param: PiParam) {
        self.param = param;
    }

    #[inline(always)]
    pub fn step(&mut self, desired: f32, measured: f32) -> f32 {
        let error = desired - measured;
        self.ki_sum = fsat(self.ki_sum + self.param.ki * error, self.param.ki_limit);
        fsat(self.param.kp * error + self.ki_sum, self.param.command_max)
    }

    pub const fn ki_sum(self) -> f32 {
        self.ki_sum
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstOrderLowPassFilter {
    dt: f32,
    alpha: f32,
    value: f32,
}

impl FirstOrderLowPassFilter {
    pub fn new(dt: f32, frequency_hz: f32) -> Self {
        let mut filter = Self {
            dt,
            alpha: 1.0,
            value: 0.0,
        };
        filter.set_frequency(frequency_hz);
        filter
    }

    #[inline(always)]
    pub fn init(&mut self, value: f32) {
        self.value = value;
    }

    #[inline(always)]
    pub fn update(&mut self, value: f32) -> f32 {
        self.value += self.alpha * (value - self.value);
        self.value
    }

    #[inline(always)]
    pub fn set_frequency(&mut self, frequency_hz: f32) {
        if frequency_hz == 0.0 {
            self.alpha = 1.0;
            return;
        }

        let numerator = TWO_PI * self.dt * frequency_hz;
        self.alpha = numerator / (numerator + 1.0);
    }

    pub const fn value(self) -> f32 {
        self.value
    }

    pub const fn alpha(self) -> f32 {
        self.alpha
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocDesired {
    pub i_d: f32,
    pub i_q: f32,
    pub v_q: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocMeasured {
    pub currents: PhaseCurrents,
    pub motor_electrical_angle: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocCommand {
    pub desired: FocDesired,
    pub measured: FocMeasured,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DqCurrents {
    pub i_d: f32,
    pub i_q: f32,
    pub i_0: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocVoltages {
    pub v_a: f32,
    pub v_b: f32,
    pub v_c: f32,
    pub v_d: f32,
    pub v_q: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocStatus {
    pub desired: FocDesired,
    pub measured: DqCurrents,
    pub command: FocVoltages,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocController {
    param: FocParam,
    pi_d: PiController,
    pi_q: PiController,
    id_filter: FirstOrderLowPassFilter,
    iq_filter: FirstOrderLowPassFilter,
    i_gain: f32,
}

impl FocController {
    pub fn new(param: FocParam, dt: f32) -> Self {
        Self {
            param,
            pi_d: PiController::new(param.pi_d),
            pi_q: PiController::new(param.pi_q),
            id_filter: FirstOrderLowPassFilter::new(dt, param.current_filter_frequency_hz),
            iq_filter: FirstOrderLowPassFilter::new(dt, param.current_filter_frequency_hz),
            i_gain: 0.0,
        }
    }

    pub fn set_param(&mut self, param: FocParam) {
        self.param = param;
        self.pi_d.set_param(param.pi_d);
        self.pi_q.set_param(param.pi_q);
        self.id_filter
            .set_frequency(param.current_filter_frequency_hz);
        self.iq_filter
            .set_frequency(param.current_filter_frequency_hz);
    }

    #[inline(always)]
    pub fn current_mode(&mut self) {
        self.i_gain = 1.0;
        self.pi_d.initialize();
        self.pi_q.initialize();
    }

    #[inline(always)]
    pub fn voltage_mode(&mut self) {
        self.i_gain = 0.0;
        self.pi_d.initialize();
        self.pi_q.initialize();
    }

    #[inline(always)]
    pub fn step_with_sincos(&mut self, command: &FocCommand, sin_t: f32, cos_t: f32) -> FocStatus {
        let (measured, voltage_command) =
            self.step_parts(command.desired, command.measured.currents, sin_t, cos_t);
        FocStatus {
            desired: command.desired,
            measured,
            command: voltage_command,
        }
    }

    #[inline(always)]
    pub fn step_voltage_command_with_sincos(
        &mut self,
        desired: FocDesired,
        currents: PhaseCurrents,
        sin_t: f32,
        cos_t: f32,
    ) -> FocVoltages {
        let (_, voltage_command) = self.step_parts(desired, currents, sin_t, cos_t);
        voltage_command
    }

    #[inline(always)]
    fn step_parts(
        &mut self,
        desired: FocDesired,
        currents: PhaseCurrents,
        sin_t: f32,
        cos_t: f32,
    ) -> (DqCurrents, FocVoltages) {
        let i_alpha = TWO_THIRDS * currents.phase_a
            + NEG_ONE_THIRD * currents.phase_b
            + NEG_ONE_THIRD * currents.phase_c;
        let i_beta = ONE_OVER_SQRT3 * currents.phase_b - ONE_OVER_SQRT3 * currents.phase_c;

        let i_d = cos_t * i_alpha - sin_t * i_beta;
        let i_q = sin_t * i_alpha + cos_t * i_beta;
        let i_d_filtered = self.id_filter.update(i_d);
        let i_q_filtered = self.iq_filter.update(i_q);

        let v_d = self.i_gain * self.pi_d.step(desired.i_d, i_d_filtered);
        let v_q = self.i_gain * self.pi_q.step(desired.i_q, i_q_filtered) + desired.v_q;

        let v_alpha = cos_t * v_d + sin_t * v_q;
        let v_beta = -sin_t * v_d + cos_t * v_q;
        let v_a = TWO_THIRDS * v_alpha;
        let v_b = NEG_ONE_THIRD * v_alpha + ONE_OVER_SQRT3 * v_beta;
        let v_c = NEG_ONE_THIRD * v_alpha - ONE_OVER_SQRT3 * v_beta;

        (
            DqCurrents {
                i_d,
                i_q,
                i_0: currents.phase_a + currents.phase_b + currents.phase_c,
            },
            FocVoltages {
                v_a,
                v_b,
                v_c,
                v_d,
                v_q,
            },
        )
    }
}

#[inline(always)]
pub fn fsat(value: f32, saturation: f32) -> f32 {
    let clamped_high = if value > saturation {
        saturation
    } else {
        value
    };
    if clamped_high < -saturation {
        -saturation
    } else {
        clamped_high
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-5;
    const DT_50_KHZ: f32 = 1.0 / 50_000.0;

    #[test]
    fn low_pass_filter_matches_cpp_alpha_formula() {
        let filter = FirstOrderLowPassFilter::new(DT_50_KHZ, 20_000.0);
        let numerator = TWO_PI * DT_50_KHZ * 20_000.0;

        assert_close(filter.alpha(), numerator / (numerator + 1.0));
        assert_close(filter.alpha(), 0.715_365);
    }

    #[test]
    fn pi_controller_saturates_integrator_and_output_symmetrically() {
        let mut pi = PiController::new(PiParam {
            kp: 3.0,
            ki: 2.0,
            ki_limit: 5.0,
            command_max: 6.0,
        });

        assert_close(pi.step(10.0, 0.0), 6.0);
        assert_close(pi.ki_sum(), 5.0);
        assert_close(pi.step(-10.0, 0.0), -6.0);
        assert_close(pi.ki_sum(), -5.0);
    }
    #[test]
    fn clarke_park_at_zero_angle_maps_balanced_a_axis_current_to_d_axis() {
        let mut foc = FocController::new(FocParam::MOTOR_HALL, DT_50_KHZ);
        foc.voltage_mode();

        let status = foc.step_with_sincos(
            &FocCommand {
                measured: FocMeasured {
                    currents: PhaseCurrents {
                        phase_a: 1.0,
                        phase_b: -0.5,
                        phase_c: -0.5,
                    },
                    motor_electrical_angle: 0.0,
                },
                desired: FocDesired::default(),
            },
            0.0,
            1.0,
        );

        assert_close(status.measured.i_d, 1.0);
        assert_close(status.measured.i_q, 0.0);
        assert_close(status.measured.i_0, 0.0);
    }

    #[test]
    fn zero_command_current_mode_generates_expected_first_step_voltage() {
        let mut foc = FocController::new(FocParam::MOTOR_HALL, DT_50_KHZ);
        foc.current_mode();

        let status = foc.step_with_sincos(
            &FocCommand {
                measured: FocMeasured {
                    currents: PhaseCurrents {
                        phase_a: 1.0,
                        phase_b: -0.5,
                        phase_c: -0.5,
                    },
                    motor_electrical_angle: 0.0,
                },
                desired: FocDesired::default(),
            },
            0.0,
            1.0,
        );

        let filtered_id = FirstOrderLowPassFilter::new(DT_50_KHZ, 20_000.0).alpha();
        let expected_vd =
            -(FocParam::MOTOR_HALL.pi_d.kp + FocParam::MOTOR_HALL.pi_d.ki) * filtered_id;

        assert_close(status.command.v_d, expected_vd);
        assert_close(status.command.v_q, 0.0);
        assert_close(status.command.v_a, TWO_THIRDS * expected_vd);
        assert_close(status.command.v_b, NEG_ONE_THIRD * expected_vd);
        assert_close(status.command.v_c, NEG_ONE_THIRD * expected_vd);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "actual={actual}, expected={expected}"
        );
    }
}
