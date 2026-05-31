use core::ptr::{read_volatile, write_volatile};

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
const HRTIM_COUNTS_PER_US: u32 = CPU_HZ / 1_000_000 * HRTIM_PRESCALER / 4;
const DEADTIME_COUNTS: u32 = DEADTIME_NS * HRTIM_COUNTS_PER_US / 1_000;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PwmConfig {
    pub frequency_hz: u32,
    pub period_counts: u32,
    pub zero_compare_counts: u32,
    pub deadtime_counts: u32,
}

impl PwmConfig {
    pub const MOTOR_HALL_SAFE_ZERO: Self = Self {
        frequency_hz: PWM_HZ,
        period_counts: PWM_PERIOD,
        zero_compare_counts: PWM_ZERO_COMPARE,
        deadtime_counts: DEADTIME_COUNTS,
    };
}

pub struct SafeZeroPwm {
    config: PwmConfig,
}

impl SafeZeroPwm {
    pub fn init_motor_hall() -> Self {
        enable_hrtim_clock();
        disable_outputs();
        let pwm = Self {
            config: PwmConfig::MOTOR_HALL_SAFE_ZERO,
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

    pub fn write_zero_voltage(&self) {
        self.write_phase_zero(TIMER_D);
        self.write_phase_zero(TIMER_E);
        self.write_phase_zero(TIMER_F);
    }

    pub fn config(&self) -> PwmConfig {
        self.config
    }

    fn configure_timer(&self, timer: usize) {
        write(timer_register(timer, HRTIM_TIMXCR2), HRTIM_TIMCR2_UDM);
        write(
            timer_register(timer, HRTIM_PERXR),
            self.config.period_counts,
        );
        write(
            timer_register(timer, HRTIM_CMP1XR),
            self.config.zero_compare_counts,
        );
        write(timer_register(timer, HRTIM_OUTXR), HRTIM_OUTR_DTEN);
        write(
            timer_register(timer, HRTIM_DTXR),
            deadtime_register(self.config),
        );
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

    fn write_phase_zero(&self, timer: usize) {
        write(
            timer_register(timer, HRTIM_CMP1XR),
            self.config.zero_compare_counts,
        );
    }
}

fn enable_hrtim_clock() {
    modify(RCC_APB2ENR, |value| value | RCC_APB2ENR_HRTIM1EN);
    let _ = read(RCC_APB2ENR);
}

fn disable_outputs() {
    write(HRTIM_COMMON_ODISR, HRTIM_DISABLE_ALL_OUTPUTS);
}

fn start_motor_timers() {
    modify(HRTIM_MASTER_MCR, |value| {
        value | HRTIM_MCR_TDCEN | HRTIM_MCR_TECEN | HRTIM_MCR_TFCEN
    });
}

fn deadtime_register(config: PwmConfig) -> u32 {
    (config.deadtime_counts << HRTIM_DTR_DTF_POS) | (config.deadtime_counts << HRTIM_DTR_DTR_POS)
}

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

fn write(address: usize, value: u32) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    // Volatile access is required so register writes are performed as requested.
    unsafe { write_volatile(address as *mut u32, value) };
}
