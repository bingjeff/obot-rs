#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RawCurrentSamples {
    pub phase_a: u16,
    pub phase_b: u16,
    pub phase_c: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhaseCurrents {
    pub phase_a: f32,
    pub phase_b: f32,
    pub phase_c: f32,
}

const MOTOR_HALL_CURRENT_GAIN: f32 = -3.3 / 4096.0 / (0.005 * 40.0);
const MOTOR_HALL_BIAS_A: f32 = 0.321;
const MOTOR_HALL_BIAS_B: f32 = 0.576;
const MOTOR_HALL_BIAS_C: f32 = 0.263;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurrentCalibration {
    pub gain_a: f32,
    pub gain_b: f32,
    pub gain_c: f32,
    pub bias_a: f32,
    pub bias_b: f32,
    pub bias_c: f32,
}

impl CurrentCalibration {
    pub const MOTOR_HALL: Self = Self {
        gain_a: MOTOR_HALL_CURRENT_GAIN,
        gain_b: MOTOR_HALL_CURRENT_GAIN,
        gain_c: MOTOR_HALL_CURRENT_GAIN,
        bias_a: MOTOR_HALL_BIAS_A,
        bias_b: MOTOR_HALL_BIAS_B,
        bias_c: MOTOR_HALL_BIAS_C,
    };

    #[inline(always)]
    pub fn motor_hall_convert(raw: RawCurrentSamples) -> PhaseCurrents {
        PhaseCurrents {
            phase_a: convert_one(raw.phase_a, MOTOR_HALL_CURRENT_GAIN, MOTOR_HALL_BIAS_A),
            phase_b: convert_one(raw.phase_b, MOTOR_HALL_CURRENT_GAIN, MOTOR_HALL_BIAS_B),
            phase_c: convert_one(raw.phase_c, MOTOR_HALL_CURRENT_GAIN, MOTOR_HALL_BIAS_C),
        }
    }

    pub fn convert(self, raw: RawCurrentSamples) -> PhaseCurrents {
        PhaseCurrents {
            phase_a: convert_one(raw.phase_a, self.gain_a, self.bias_a),
            phase_b: convert_one(raw.phase_b, self.gain_b, self.bias_b),
            phase_c: convert_one(raw.phase_c, self.gain_c, self.bias_c),
        }
    }

    pub fn zero_update(&mut self, raw: RawCurrentSamples, alpha: f32) {
        self.bias_a = zero_update_one(self.bias_a, self.gain_a, raw.phase_a, alpha);
        self.bias_b = zero_update_one(self.bias_b, self.gain_b, raw.phase_b, alpha);
        self.bias_c = zero_update_one(self.bias_c, self.gain_c, raw.phase_c, alpha);
    }
}

fn convert_one(raw: u16, gain: f32, bias: f32) -> f32 {
    gain * (raw as f32 - 2048.0) - bias
}

fn zero_update_one(bias: f32, gain: f32, raw: u16, alpha: f32) -> f32 {
    (1.0 - alpha) * bias + alpha * gain * (raw as f32 - 2048.0)
}

#[cfg(test)]
mod tests {
    use super::{CurrentCalibration, PhaseCurrents, RawCurrentSamples};

    const EPSILON: f32 = 1.0e-6;

    #[test]
    fn motor_hall_constants_match_c_param_shape() {
        let calibration = CurrentCalibration::MOTOR_HALL;
        let expected_gain = -3.3 / 4096.0 / (0.005 * 40.0);
        assert_close(calibration.gain_a, expected_gain);
        assert_close(calibration.gain_b, expected_gain);
        assert_close(calibration.gain_c, expected_gain);
        assert_close(calibration.bias_a, 0.321);
        assert_close(calibration.bias_b, 0.576);
        assert_close(calibration.bias_c, 0.263);
    }

    #[test]
    fn converts_raw_samples_like_cpp_fast_loop() {
        let calibration = CurrentCalibration::MOTOR_HALL;
        let currents = calibration.convert(RawCurrentSamples {
            phase_a: 0x0331,
            phase_b: 0x069f,
            phase_c: 0x05fc,
        });
        let expected = PhaseCurrents {
            phase_a: calibration.gain_a * (0x0331 as f32 - 2048.0) - calibration.bias_a,
            phase_b: calibration.gain_b * (0x069f as f32 - 2048.0) - calibration.bias_b,
            phase_c: calibration.gain_c * (0x05fc as f32 - 2048.0) - calibration.bias_c,
        };
        assert_currents_close(currents, expected);
        assert_currents_close(
            CurrentCalibration::motor_hall_convert(RawCurrentSamples {
                phase_a: 0x0331,
                phase_b: 0x069f,
                phase_c: 0x05fc,
            }),
            expected,
        );
    }

    #[test]
    fn zero_current_update_matches_cpp_formula() {
        let mut calibration = CurrentCalibration::MOTOR_HALL;
        calibration.zero_update(
            RawCurrentSamples {
                phase_a: 2100,
                phase_b: 2000,
                phase_c: 2048,
            },
            0.25,
        );
        assert_close(
            calibration.bias_a,
            0.75 * 0.321 + 0.25 * calibration.gain_a * (2100.0 - 2048.0),
        );
        assert_close(
            calibration.bias_b,
            0.75 * 0.576 + 0.25 * calibration.gain_b * (2000.0 - 2048.0),
        );
        assert_close(calibration.bias_c, 0.75 * 0.263);
    }

    fn assert_currents_close(actual: PhaseCurrents, expected: PhaseCurrents) {
        assert_close(actual.phase_a, expected.phase_a);
        assert_close(actual.phase_b, expected.phase_b);
        assert_close(actual.phase_c, expected.phase_c);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "actual={actual}, expected={expected}"
        );
    }
}
