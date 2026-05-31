const HALL_TABLE: [u8; 8] = [0, 1, 3, 2, 5, 6, 4, 0];

const SQRT3_OVER_2: f32 = 0.866_025_4;
const ELECTRICAL_RADIANS_PER_MOTOR_HALL_COUNT: f32 = core::f32::consts::PI / 3.0;
const TWO_PI: f32 = 2.0 * core::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sincos {
    pub sin: f32,
    pub cos: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HallElectricalAngle {
    phase_sign: i32,
    zero_count: i32,
}

impl HallElectricalAngle {
    pub const MOTOR_HALL: Self = Self {
        phase_sign: -1,
        zero_count: 0,
    };

    pub const fn new(phase_sign: i32, zero_count: i32) -> Self {
        Self {
            phase_sign,
            zero_count,
        }
    }

    #[inline(always)]
    pub fn electrical_radians(self, count: i32) -> f32 {
        self.signed_electrical_count(count) as f32 * ELECTRICAL_RADIANS_PER_MOTOR_HALL_COUNT
    }

    #[inline(always)]
    pub fn sincos(self, count: i32) -> Sincos {
        self.sincos_sector(rem_euclid_6(self.signed_electrical_count(count)) as u8)
    }

    #[inline(always)]
    pub fn sincos_hall_count(self, hall_count: u8) -> Sincos {
        let sector = if hall_count == 0 {
            0
        } else if self.phase_sign < 0 {
            6 - hall_count
        } else if hall_count == 6 {
            0
        } else {
            hall_count
        };
        self.sincos_sector(sector)
    }

    #[inline(always)]
    pub fn motor_hall_sincos_hall_count(hall_count: u8) -> Sincos {
        match hall_count {
            1 => Sincos {
                sin: -SQRT3_OVER_2,
                cos: 0.5,
            },
            2 => Sincos {
                sin: -SQRT3_OVER_2,
                cos: -0.5,
            },
            3 => Sincos {
                sin: 0.0,
                cos: -1.0,
            },
            4 => Sincos {
                sin: SQRT3_OVER_2,
                cos: -0.5,
            },
            5 => Sincos {
                sin: SQRT3_OVER_2,
                cos: 0.5,
            },
            _ => Sincos { sin: 0.0, cos: 1.0 },
        }
    }

    #[inline(always)]
    fn sincos_sector(self, sector: u8) -> Sincos {
        match sector {
            0 => Sincos { sin: 0.0, cos: 1.0 },
            1 => Sincos {
                sin: SQRT3_OVER_2,
                cos: 0.5,
            },
            2 => Sincos {
                sin: SQRT3_OVER_2,
                cos: -0.5,
            },
            3 => Sincos {
                sin: 0.0,
                cos: -1.0,
            },
            4 => Sincos {
                sin: -SQRT3_OVER_2,
                cos: -0.5,
            },
            _ => Sincos {
                sin: -SQRT3_OVER_2,
                cos: 0.5,
            },
        }
    }

    #[inline(always)]
    fn signed_electrical_count(self, count: i32) -> i32 {
        self.phase_sign * (count - self.zero_count)
    }
}

#[inline(always)]
fn rem_euclid_6(value: i32) -> i32 {
    let remainder = value % 6;
    if remainder < 0 {
        remainder + 6
    } else {
        remainder
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HallSample {
    pub count: i32,
    pub hall_count: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HallMotionEstimate {
    pub raw_count: i32,
    pub position_rad: f32,
    pub velocity_rad_s: f32,
    pub velocity_filtered_rad_s: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HallMotionEstimator {
    position_radians_per_count: f32,
    velocity_radians_per_count: f32,
    velocity_filter_alpha: f32,
    last_count: i32,
    initialized: bool,
    estimate: HallMotionEstimate,
}

impl HallMotionEstimator {
    pub const MOTOR_HALL: Self = Self::new(42.0, -1.0, 50_000.0, 0.001);

    pub const fn new(
        counts_per_revolution: f32,
        direction: f32,
        sample_frequency_hz: f32,
        velocity_filter_alpha: f32,
    ) -> Self {
        let position_radians_per_count = direction * TWO_PI / counts_per_revolution;
        Self {
            position_radians_per_count,
            velocity_radians_per_count: position_radians_per_count * sample_frequency_hz,
            velocity_filter_alpha,
            last_count: 0,
            initialized: false,
            estimate: HallMotionEstimate {
                raw_count: 0,
                position_rad: 0.0,
                velocity_rad_s: 0.0,
                velocity_filtered_rad_s: 0.0,
            },
        }
    }

    #[inline(always)]
    pub fn update(&mut self, sample: HallSample) -> HallMotionEstimate {
        let diff = if self.initialized {
            sample.count - self.last_count
        } else {
            self.initialized = true;
            0
        };
        if diff == 0
            && self.estimate.raw_count == sample.count
            && self.estimate.velocity_filtered_rad_s == 0.0
        {
            return self.estimate;
        }
        self.last_count = sample.count;

        let position_rad = sample.count as f32 * self.position_radians_per_count;
        let velocity_rad_s = diff as f32 * self.velocity_radians_per_count;
        let velocity_filtered_rad_s = self.estimate.velocity_filtered_rad_s
            + self.velocity_filter_alpha * (velocity_rad_s - self.estimate.velocity_filtered_rad_s);

        self.estimate = HallMotionEstimate {
            raw_count: sample.count,
            position_rad,
            velocity_rad_s,
            velocity_filtered_rad_s,
        };
        self.estimate
    }

    pub const fn estimate(&self) -> HallMotionEstimate {
        self.estimate
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HallEncoder {
    count: i32,
    last_hall_count: u8,
}

impl HallEncoder {
    pub const fn new() -> Self {
        Self {
            count: 0,
            last_hall_count: 0,
        }
    }

    #[inline(always)]
    pub fn read(&mut self, hall_bits: u8) -> i32 {
        self.read_sample(hall_bits).count
    }

    #[inline(always)]
    pub fn read_sample(&mut self, hall_bits: u8) -> HallSample {
        let hall_count = HALL_TABLE[(hall_bits & 0x07) as usize];
        if hall_count != 0 {
            let mut diff = hall_count as i8 - self.last_hall_count as i8;
            self.last_hall_count = hall_count;
            if diff < -3 {
                diff += 6;
            } else if diff > 3 {
                diff -= 6;
            }
            self.count += diff as i32;
        }
        HallSample {
            count: self.count,
            hall_count: self.last_hall_count,
        }
    }

    pub fn count(&self) -> i32 {
        self.count
    }
}

impl Default for HallEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{HallEncoder, HallMotionEstimator};

    #[test]
    fn follows_cpp_forward_hall_table() {
        let mut encoder = HallEncoder::new();
        for (raw, expected) in [(1, 1), (3, 2), (2, 3), (6, 4), (4, 5), (5, 6), (1, 7)] {
            assert_eq!(encoder.read(raw), expected);
        }
        assert_eq!(encoder.count(), 7);
    }

    #[test]
    fn wraps_reverse_hall_steps() {
        let mut encoder = HallEncoder::new();
        encoder.read(1);
        assert_eq!(encoder.read(5), 0);
        assert_eq!(encoder.read(4), -1);
        assert_eq!(encoder.read(6), -2);
    }

    #[test]
    fn ignores_invalid_zero_and_seven_states() {
        let mut encoder = HallEncoder::new();
        assert_eq!(encoder.read(0), 0);
        assert_eq!(encoder.read(7), 0);
        assert_eq!(encoder.read(1), 1);
        assert_eq!(encoder.read(7), 1);
    }

    #[test]
    fn motor_hall_angle_uses_phase_mode_one_sign() {
        let angle = super::HallElectricalAngle::MOTOR_HALL;

        assert_close(angle.electrical_radians(1), -core::f32::consts::PI / 3.0);
        assert_close(angle.sincos(0).sin, 0.0);
        assert_close(angle.sincos(0).cos, 1.0);
        assert_close(angle.sincos(1).sin, -0.866_025_4);
        assert_close(angle.sincos(1).cos, 0.5);
        assert_close(angle.sincos(2).sin, -0.866_025_4);
        assert_close(angle.sincos(2).cos, -0.5);
        assert_close(angle.sincos(3).sin, 0.0);
        assert_close(angle.sincos(3).cos, -1.0);
        assert_close(angle.sincos_hall_count(1).sin, -0.866_025_4);
        assert_close(angle.sincos_hall_count(1).cos, 0.5);
        assert_close(angle.sincos_hall_count(6).sin, 0.0);
        assert_close(angle.sincos_hall_count(6).cos, 1.0);
        for hall_count in 0..=6 {
            assert_eq!(
                super::HallElectricalAngle::motor_hall_sincos_hall_count(hall_count),
                angle.sincos_hall_count(hall_count)
            );
        }
    }

    #[test]
    fn read_sample_exposes_last_valid_hall_sector() {
        let mut encoder = HallEncoder::new();
        assert_eq!(encoder.read_sample(0).hall_count, 0);
        let sample = encoder.read_sample(1);
        assert_eq!(sample.count, 1);
        assert_eq!(sample.hall_count, 1);
        let invalid = encoder.read_sample(7);
        assert_eq!(invalid.count, 1);
        assert_eq!(invalid.hall_count, 1);
    }

    #[test]
    fn hall_motion_estimator_matches_motor_hall_scale_and_direction() {
        let mut estimator = HallMotionEstimator::MOTOR_HALL;

        let first = estimator.update(super::HallSample {
            count: 1,
            hall_count: 1,
        });
        assert_eq!(first.raw_count, 1);
        assert_close(first.position_rad, -2.0 * core::f32::consts::PI / 42.0);
        assert_close(first.velocity_rad_s, 0.0);
        assert_close(first.velocity_filtered_rad_s, 0.0);

        let second = estimator.update(super::HallSample {
            count: 2,
            hall_count: 2,
        });
        let expected_velocity = -2.0 * core::f32::consts::PI / 42.0 * 50_000.0;
        assert_close(second.position_rad, -4.0 * core::f32::consts::PI / 42.0);
        assert_close(second.velocity_rad_s, expected_velocity);
        assert_close(second.velocity_filtered_rad_s, expected_velocity * 0.001);
    }

    #[test]
    fn hall_motion_estimator_keeps_stationary_zero_velocity_estimate() {
        let mut estimator = HallMotionEstimator::MOTOR_HALL;
        let first = estimator.update(super::HallSample {
            count: 3,
            hall_count: 3,
        });
        let second = estimator.update(super::HallSample {
            count: 3,
            hall_count: 3,
        });

        assert_eq!(second, first);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "actual={actual}, expected={expected}"
        );
    }
}
