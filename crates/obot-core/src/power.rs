use crate::foc::FocVoltages;

const ADC_VREF_V: f32 = 3.3;
const MOTOR_HALL_VBUS_DIVIDER_GAIN: f32 = (215.0 + 13.7) / 13.7;
const MOTOR_HALL_VBUS_VOLTS_PER_COUNT: f32 = ADC_VREF_V * MOTOR_HALL_VBUS_DIVIDER_GAIN / 4096.0;
const MOTOR_HALL_VBUS_MIN_RAW: u16 = 595;
const MOTOR_HALL_VBUS_MAX_RAW: u16 = 4_095;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BusVoltageSample {
    pub raw: u16,
    pub volts: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BusVoltageCalibration {
    pub volts_per_count: f32,
}

impl BusVoltageCalibration {
    pub const MOTOR_HALL: Self = Self {
        volts_per_count: MOTOR_HALL_VBUS_VOLTS_PER_COUNT,
    };

    #[inline(always)]
    pub fn convert(self, raw: u16) -> BusVoltageSample {
        BusVoltageSample {
            raw,
            volts: raw as f32 * self.volts_per_count,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputGate {
    pub min_vbus_v: f32,
    pub max_vbus_v: f32,
    pub min_raw: u16,
    pub max_raw: u16,
}

impl OutputGate {
    pub const MOTOR_HALL: Self = Self {
        min_vbus_v: 8.0,
        max_vbus_v: 60.0,
        min_raw: MOTOR_HALL_VBUS_MIN_RAW,
        max_raw: MOTOR_HALL_VBUS_MAX_RAW,
    };

    #[inline(always)]
    pub fn allows_output(self, bus: BusVoltageSample) -> bool {
        bus.volts >= self.min_vbus_v && bus.volts <= self.max_vbus_v
    }

    #[inline(always)]
    pub fn allows_output_raw(self, raw: u16) -> bool {
        raw >= self.min_raw && raw <= self.max_raw
    }

    #[inline(always)]
    pub fn gate_voltages(self, command: FocVoltages, bus: BusVoltageSample) -> FocVoltages {
        if self.allows_output(bus) {
            command
        } else {
            FocVoltages::default()
        }
    }

    #[inline(always)]
    pub fn gate_voltages_raw(self, command: FocVoltages, raw: u16) -> FocVoltages {
        if self.allows_output_raw(raw) {
            command
        } else {
            FocVoltages::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-6;

    #[test]
    fn motor_hall_vbus_gain_matches_cpp_param() {
        let calibration = BusVoltageCalibration::MOTOR_HALL;
        assert_close(
            calibration.volts_per_count,
            ADC_VREF_V * (215.0 + 13.7) / 13.7 / 4096.0,
        );
        assert_close(
            calibration.convert(4096).volts,
            ADC_VREF_V * (215.0 + 13.7) / 13.7,
        );
        assert_close(calibration.convert(3_521).volts, 47.354_973);
    }

    #[test]
    fn motor_hall_output_gate_uses_cpp_threshold_shape() {
        let gate = OutputGate::MOTOR_HALL;
        assert!(!gate.allows_output(BusVoltageSample {
            raw: 0,
            volts: 7.99
        }));
        assert!(gate.allows_output(BusVoltageSample { raw: 0, volts: 8.0 }));
        assert!(gate.allows_output(BusVoltageSample {
            raw: 0,
            volts: 60.0
        }));
        assert!(!gate.allows_output(BusVoltageSample {
            raw: 0,
            volts: 60.01
        }));
    }

    #[test]
    fn output_gate_zeros_voltage_commands_outside_bus_limits() {
        let command = FocVoltages {
            v_a: 1.0,
            v_b: -2.0,
            v_c: 0.5,
            v_d: 0.25,
            v_q: -0.25,
        };
        let gate = OutputGate::MOTOR_HALL;

        assert_eq!(
            gate.gate_voltages(
                command,
                BusVoltageSample {
                    raw: 0,
                    volts: 12.0
                }
            ),
            command
        );
        assert_eq!(
            gate.gate_voltages(command, BusVoltageSample { raw: 0, volts: 0.0 }),
            FocVoltages::default()
        );
        assert_eq!(gate.gate_voltages_raw(command, 595), command);
        assert_eq!(gate.gate_voltages_raw(command, 594), FocVoltages::default());
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "actual={actual}, expected={expected}"
        );
    }
}
