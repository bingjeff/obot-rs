use core::ptr::{read_volatile, write_volatile};

use obot_core::foc::FocVoltages;

const RCC_BASE: usize = 0x4002_1000;
const RCC_APB2ENR: usize = RCC_BASE + 0x60;
const RCC_APB2ENR_HRTIM1EN: u32 = 1 << 26;

const HRTIM1_BASE: usize = 0x4001_6800;
const HRTIM_MASTER_MCR: usize = HRTIM1_BASE;
const HRTIM_COMMON_BASE: usize = HRTIM1_BASE + 0x380;
const HRTIM_COMMON_ADC1R: usize = HRTIM_COMMON_BASE + 0x3C;
const HRTIM_COMMON_ADC2R: usize = HRTIM_COMMON_BASE + 0x40;
const HRTIM_COMMON_DLLCR: usize = HRTIM_COMMON_BASE + 0x4C;
const HRTIM_COMMON_ODISR: usize = HRTIM_COMMON_BASE + 0x18;
const HRTIM_COMMON_ODSR: usize = HRTIM_COMMON_BASE + 0x1C;

const HRTIM_TIMER_BASE: usize = HRTIM1_BASE + 0x80;
const HRTIM_TIMER_STRIDE: usize = 0x80;
const HRTIM_TIMXCR: usize = 0x00;
const HRTIM_PERXR: usize = 0x14;
const HRTIM_CMP1XR: usize = 0x1C;
const HRTIM_DTXR: usize = 0x38;
const HRTIM_OUTXR: usize = 0x64;
const HRTIM_TIMXCR2: usize = 0x6C;

const TIMER_D: usize = 3;
const TIMER_E: usize = 4;
const TIMER_F: usize = 5;

const CPU_HZ: u32 = 170_000_000;
const PWM_HZ: u32 = 50_000;
const HRTIM_PRESCALER: u32 = 32;
const PWM_PERIOD: u32 = ((CPU_HZ as u64 * HRTIM_PRESCALER as u64) / 2 / PWM_HZ as u64) as u32;
const PWM_ZERO_COMPARE: u32 = PWM_PERIOD / 2;
const DEADTIME_NS: u32 = 200;
const MIN_OFF_NS: u32 = 1_000;
const MIN_ON_NS: u32 = 0;
const NOMINAL_VBUS_V: f32 = 12.0;
const HRTIM_COUNTS_PER_US: u32 = CPU_HZ / 1_000_000 * HRTIM_PRESCALER / 4;
const DEADTIME_COUNTS: u32 = DEADTIME_NS * HRTIM_COUNTS_PER_US / 1_000;
const MIN_OFF_COUNTS: u32 = 2 * MIN_OFF_NS * HRTIM_COUNTS_PER_US / 1_000;
const MIN_ON_COUNTS: u32 = 2 * MIN_ON_NS * HRTIM_COUNTS_PER_US / 1_000;
const PWM_MIN_COMPARE: u32 = MIN_OFF_COUNTS;
const PWM_MAX_COMPARE: u32 = PWM_PERIOD - max_u32(MIN_ON_COUNTS, 65);
const PWM_ZERO_COMPARE_F32: f32 = PWM_ZERO_COMPARE as f32;
const PWM_MIN_COMPARE_I32: i32 = PWM_MIN_COMPARE as i32;
const PWM_MAX_COMPARE_I32: i32 = PWM_MAX_COMPARE as i32;
const PWM_COUNTS_PER_VOLT: f32 = PWM_PERIOD as f32 / NOMINAL_VBUS_V;

const HRTIM_TIMCR_CONT: u32 = 1 << 3;
const HRTIM_TIMCR_TRSTU: u32 = 1 << 18;
const HRTIM_TIMCR_PREEN: u32 = 1 << 27;
const HRTIM_TIMCR_RUN: u32 = HRTIM_TIMCR_CONT | HRTIM_TIMCR_TRSTU | HRTIM_TIMCR_PREEN;
const HRTIM_TIMCR2_UDM: u32 = 1 << 4;
const HRTIM_TIMCR2_ADROM_VALLEY: u32 = 1 << 10;
const HRTIM_OUTR_DTEN: u32 = 1 << 8;
const HRTIM_DTR_DTR_POS: u32 = 0;
const HRTIM_DTR_DTF_POS: u32 = 16;
const HRTIM_MCR_TDCEN: u32 = 1 << 20;
const HRTIM_MCR_TECEN: u32 = 1 << 21;
const HRTIM_MCR_TFCEN: u32 = 1 << 22;
const HRTIM_DLLCR_CALEN: u32 = 1 << 1;
const HRTIM_DLLCR_CALRTE_2048: u32 = 3 << 2;
const HRTIM_ADC_TRIGGER_TIMER_F_PERIOD: u32 = 1 << 24;
const HRTIM_DISABLE_ALL_OUTPUTS: u32 = 0x0FFF;
const HRTIM_MOTOR_OUTPUTS: u32 = 0x0FC0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PwmConfig {
    pub frequency_hz: u32,
    pub period_counts: u32,
    pub zero_compare_counts: u32,
    pub deadtime_counts: u32,
    pub min_compare_counts: u32,
    pub max_compare_counts: u32,
    pub zero_compare_counts_f32: f32,
    pub min_compare_counts_i32: i32,
    pub max_compare_counts_i32: i32,
    pub nominal_vbus_v: f32,
    pub counts_per_volt: f32,
}

impl PwmConfig {
    pub const MOTOR_HALL_SAFE_ZERO: Self = Self {
        frequency_hz: PWM_HZ,
        period_counts: PWM_PERIOD,
        zero_compare_counts: PWM_ZERO_COMPARE,
        deadtime_counts: DEADTIME_COUNTS,
        min_compare_counts: PWM_MIN_COMPARE,
        max_compare_counts: PWM_MAX_COMPARE,
        zero_compare_counts_f32: PWM_ZERO_COMPARE_F32,
        min_compare_counts_i32: PWM_MIN_COMPARE_I32,
        max_compare_counts_i32: PWM_MAX_COMPARE_I32,
        nominal_vbus_v: NOMINAL_VBUS_V,
        counts_per_volt: PWM_COUNTS_PER_VOLT,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseCompares {
    pub phase_a: u32,
    pub phase_b: u32,
    pub phase_c: u32,
}

impl PhaseCompares {
    pub const fn from_compare(compare: u32) -> Self {
        Self {
            phase_a: compare,
            phase_b: compare,
            phase_c: compare,
        }
    }

    pub const fn zero_voltage() -> Self {
        Self::from_compare(PWM_ZERO_COMPARE)
    }
}

pub struct SafeZeroPwm {
    bridge_outputs_armed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BridgeOutputStatus {
    pub disable_status: u32,
    pub all_disabled: bool,
    pub all_enabled: bool,
}

impl SafeZeroPwm {
    pub fn init_motor_hall() -> Self {
        enable_hrtim_clock();
        disable_outputs();
        let pwm = Self {
            bridge_outputs_armed: false,
        };
        pwm.configure_timer(TIMER_D);
        pwm.configure_timer(TIMER_E);
        pwm.configure_timer(TIMER_F);
        pwm.configure_timer_f_adc_triggers();
        pwm.configure_common();
        pwm.write_zero_voltage();
        start_motor_timers();
        pwm
    }

    #[inline(always)]
    pub fn write_zero_voltage(&self) {
        self.write_phase_zero(TIMER_D);
        self.write_phase_zero(TIMER_E);
        self.write_phase_zero(TIMER_F);
    }

    #[inline(always)]
    pub fn write_voltage_commands_disabled(&self, command: FocVoltages) -> PhaseCompares {
        let compares = self.compares_from_voltages(command);
        self.write_phase_compares(compares);
        disable_outputs();
        compares
    }

    #[inline(always)]
    pub fn write_gated_voltage_commands_disabled(
        &self,
        command: FocVoltages,
        output_allowed: bool,
    ) -> PhaseCompares {
        let commanded_compares = self.compares_from_voltages(command);
        let applied_compares = if output_allowed {
            commanded_compares
        } else {
            PhaseCompares::zero_voltage()
        };
        self.write_phase_compares(applied_compares);
        commanded_compares
    }

    #[inline(always)]
    pub fn compares_from_voltages(&self, command: FocVoltages) -> PhaseCompares {
        PhaseCompares {
            phase_a: self.compare_from_voltage(command.v_a),
            phase_b: self.compare_from_voltage(command.v_b),
            phase_c: self.compare_from_voltage(command.v_c),
        }
    }

    pub const fn config(&self) -> PwmConfig {
        PwmConfig::MOTOR_HALL_SAFE_ZERO
    }

    pub fn bridge_output_status(&self) -> BridgeOutputStatus {
        bridge_output_status(self.bridge_outputs_armed)
    }

    fn configure_timer(&self, timer: usize) {
        write(timer_register(timer, HRTIM_TIMXCR2), HRTIM_TIMCR2_UDM);
        write(timer_register(timer, HRTIM_PERXR), PWM_PERIOD);
        write(timer_register(timer, HRTIM_CMP1XR), PWM_ZERO_COMPARE);
        write(timer_register(timer, HRTIM_OUTXR), HRTIM_OUTR_DTEN);
        write(timer_register(timer, HRTIM_DTXR), deadtime_register());
        modify(timer_register(timer, HRTIM_TIMXCR), |value| {
            value | HRTIM_TIMCR_RUN
        });
    }

    fn configure_timer_f_adc_triggers(&self) {
        modify(timer_register(TIMER_F, HRTIM_TIMXCR2), |value| {
            value | HRTIM_TIMCR2_ADROM_VALLEY
        });
        write(HRTIM_COMMON_ADC1R, HRTIM_ADC_TRIGGER_TIMER_F_PERIOD);
        write(HRTIM_COMMON_ADC2R, HRTIM_ADC_TRIGGER_TIMER_F_PERIOD);
    }

    fn configure_common(&self) {
        write(
            HRTIM_COMMON_DLLCR,
            HRTIM_DLLCR_CALEN | HRTIM_DLLCR_CALRTE_2048,
        );
        disable_outputs();
    }

    #[inline(always)]
    fn write_phase_zero(&self, timer: usize) {
        self.write_phase_compare(timer, PWM_ZERO_COMPARE);
    }

    #[inline(always)]
    fn write_phase_compares(&self, compares: PhaseCompares) {
        self.write_phase_compare(TIMER_D, compares.phase_a);
        self.write_phase_compare(TIMER_F, compares.phase_b);
        self.write_phase_compare(TIMER_E, compares.phase_c);
    }

    #[inline(always)]
    fn write_phase_compare(&self, timer: usize, compare: u32) {
        write(timer_register(timer, HRTIM_CMP1XR), compare);
    }

    #[inline(always)]
    fn compare_from_voltage(&self, voltage: f32) -> u32 {
        let scaled = voltage * PWM_COUNTS_PER_VOLT + PWM_ZERO_COMPARE_F32;
        clamp_compare(scaled, PWM_MIN_COMPARE_I32, PWM_MAX_COMPARE_I32)
    }
}

fn enable_hrtim_clock() {
    modify(RCC_APB2ENR, |value| value | RCC_APB2ENR_HRTIM1EN);
    let _ = read(RCC_APB2ENR);
}

#[inline(always)]
fn disable_outputs() {
    write(HRTIM_COMMON_ODISR, HRTIM_DISABLE_ALL_OUTPUTS);
}

#[inline(always)]
fn bridge_output_status(outputs_armed: bool) -> BridgeOutputStatus {
    let disable_status = read(HRTIM_COMMON_ODSR) & HRTIM_MOTOR_OUTPUTS;
    BridgeOutputStatus {
        disable_status,
        all_disabled: !outputs_armed,
        all_enabled: outputs_armed,
    }
}

fn start_motor_timers() {
    modify(HRTIM_MASTER_MCR, |value| {
        value | HRTIM_MCR_TDCEN | HRTIM_MCR_TECEN | HRTIM_MCR_TFCEN
    });
}

const fn max_u32(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

#[inline(always)]
fn clamp_compare(value: f32, min: i32, max: i32) -> u32 {
    debug_assert!(value.is_finite());
    debug_assert!(value > i32::MIN as f32);
    debug_assert!(value < i32::MAX as f32);

    // SAFETY: the current firmware only derives PWM commands from finite ADC
    // samples and finite constants. FOC voltage commands are bounded far inside
    // the i32 range after PWM scaling, and the integer result is clamped to the
    // valid u32 compare register range before return.
    let compare = unsafe { value.to_int_unchecked::<i32>() };
    compare.clamp(min, max) as u32
}

fn deadtime_register() -> u32 {
    (DEADTIME_COUNTS << HRTIM_DTR_DTF_POS) | (DEADTIME_COUNTS << HRTIM_DTR_DTR_POS)
}

#[inline(always)]
fn timer_register(timer: usize, offset: usize) -> usize {
    HRTIM_TIMER_BASE + timer * HRTIM_TIMER_STRIDE + offset
}

fn modify(address: usize, f: impl FnOnce(u32) -> u32) {
    let value = read(address);
    write(address, f(value));
}

fn read(address: usize) -> u32 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    // Volatile access is required so register reads are not elided or cached.
    unsafe { read_volatile(address as *const u32) }
}

#[inline(always)]
fn write(address: usize, value: u32) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    // Volatile access is required so register writes are performed as requested.
    unsafe { write_volatile(address as *mut u32, value) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_hall_pwm_config_matches_cpp_shape() {
        let config = PwmConfig::MOTOR_HALL_SAFE_ZERO;

        assert_eq!(config.period_counts, 54_400);
        assert_eq!(config.zero_compare_counts, 27_200);
        assert_eq!(config.deadtime_counts, 272);
        assert_eq!(config.min_compare_counts, 2_720);
        assert_eq!(config.max_compare_counts, 54_335);
        assert_eq!(config.zero_compare_counts_f32, 27_200.0);
        assert_eq!(config.min_compare_counts_i32, 2_720);
        assert_eq!(config.max_compare_counts_i32, 54_335);
        assert_eq!(config.nominal_vbus_v, 12.0);
        assert_eq!(config.counts_per_volt, 54_400.0 / 12.0);
    }

    #[test]
    fn zero_voltage_compares_use_centered_pwm() {
        assert_eq!(
            PhaseCompares::zero_voltage(),
            PhaseCompares {
                phase_a: PwmConfig::MOTOR_HALL_SAFE_ZERO.zero_compare_counts,
                phase_b: PwmConfig::MOTOR_HALL_SAFE_ZERO.zero_compare_counts,
                phase_c: PwmConfig::MOTOR_HALL_SAFE_ZERO.zero_compare_counts,
            }
        );
    }

    #[test]
    fn compare_from_voltage_matches_cpp_formula() {
        let config = PwmConfig::MOTOR_HALL_SAFE_ZERO;

        assert_eq!(
            clamp_compare(
                1.5 * config.counts_per_volt + config.zero_compare_counts as f32,
                config.min_compare_counts_i32,
                config.max_compare_counts_i32
            ),
            34_000,
        );
        assert_eq!(
            clamp_compare(
                -100.0 * config.counts_per_volt + config.zero_compare_counts as f32,
                config.min_compare_counts_i32,
                config.max_compare_counts_i32
            ),
            config.min_compare_counts,
        );
        assert_eq!(
            clamp_compare(
                100.0 * config.counts_per_volt + config.zero_compare_counts as f32,
                config.min_compare_counts_i32,
                config.max_compare_counts_i32
            ),
            config.max_compare_counts,
        );
    }
}
