use core::{
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
};

const RCC_BASE: usize = 0x4002_1000;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_APB2ENR: usize = RCC_BASE + 0x60;
const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
const RCC_AHB2ENR_ADC345EN: u32 = 1 << 14;
const RCC_APB2ENR_SYSCFGEN: u32 = 1 << 0;

const GPIOB_BASE: usize = 0x4800_0400;
const GPIO_MODER: usize = GPIOB_BASE;
const GPIO_OSPEEDR: usize = GPIOB_BASE + 0x08;
const GPIO_AFRH: usize = GPIOB_BASE + 0x24;
const CURRENT_PIN_MODE_MASK: u32 =
    two_bit_pin_mask(11) | two_bit_pin_mask(12) | two_bit_pin_mask(13);
const CURRENT_PIN_AFRH_MASK: u32 =
    four_bit_pin_mask(11) | four_bit_pin_mask(12) | four_bit_pin_mask(13);
const GPIO_MODE_ANALOG: u32 = 0b11;
const CURRENT_PIN_ANALOG_VALUE: u32 = (GPIO_MODE_ANALOG << (11 * 2))
    | (GPIO_MODE_ANALOG << (12 * 2))
    | (GPIO_MODE_ANALOG << (13 * 2));

const OPAMP3_BASE: usize = 0x4001_0308;
const OPAMP4_BASE: usize = 0x4001_030C;
const OPAMP6_BASE: usize = 0x4001_0314;
const OPAMP_CSR_ENABLE: u32 = 1 << 0;
const OPAMP_CSR_VPSEL_POS: u32 = 2;
const OPAMP_CSR_VMSEL_POS: u32 = 5;
const OPAMP_CSR_HIGHSPEEDEN: u32 = 1 << 7;
const OPAMP_CSR_OPAMPINTEN: u32 = 1 << 8;

const ADC3_BASE: usize = 0x5000_0400;
const ADC4_BASE: usize = 0x5000_0500;
const ADC5_BASE: usize = 0x5000_0600;
const ADC345_COMMON_BASE: usize = 0x5000_0700;
const ADC_COMMON_CCR: usize = ADC345_COMMON_BASE + 0x08;

const ADC_ISR: usize = 0x00;
const ADC_CR: usize = 0x08;
const ADC_SMPR1: usize = 0x14;
const ADC_SMPR2: usize = 0x18;
const ADC_JSQR: usize = 0x4C;
const ADC_JDR1: usize = 0x80;

const ADC_ISR_ADRDY: u32 = 1 << 0;
const ADC_CR_ADEN: u32 = 1 << 0;
const ADC_CR_JADSTART: u32 = 1 << 3;
const ADC_CR_ADVREGEN: u32 = 1 << 28;
const ADC_CR_ADCALDIF: u32 = 1 << 30;
const ADC_CR_ADCAL: u32 = 1 << 31;
const ADC_CCR_CKMODE_HCLK_DIV4: u32 = 3 << 16;
const ADC_CCR_VREFEN: u32 = 1 << 22;
const ADC_JSQR_HRTIM_TRG1: u32 = (27 << 2) | (1 << 7);
const ADC_JSQR_JSQ1_POS: u32 = 9;
const ADC_SAMPLE_12_5_CYCLES: u32 = 2;

const WAIT_ITERATIONS: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CurrentSamples {
    pub phase_a: u16,
    pub phase_b: u16,
    pub phase_c: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurrentAdcError {
    CalibrationTimeout { adc_base: usize },
    ReadyTimeout { adc_base: usize },
}

pub struct CurrentAdc;

impl CurrentAdc {
    pub fn init_motor_hall() -> Result<Self, CurrentAdcError> {
        enable_clocks();
        configure_current_opamps();
        configure_adc_common();
        configure_current_adc(ADC3_BASE, CurrentAdcChannel::PhaseA);
        configure_current_adc(ADC4_BASE, CurrentAdcChannel::PhaseB);
        configure_current_adc(ADC5_BASE, CurrentAdcChannel::PhaseC);
        enable_adc(ADC3_BASE)?;
        enable_adc(ADC4_BASE)?;
        enable_adc(ADC5_BASE)?;
        start_injected_conversions();
        Ok(Self)
    }

    pub fn read_samples(&self) -> CurrentSamples {
        CurrentSamples {
            phase_a: read_jdr1(ADC3_BASE),
            phase_b: read_jdr1(ADC4_BASE),
            phase_c: read_jdr1(ADC5_BASE),
        }
    }
}

#[derive(Clone, Copy)]
enum CurrentAdcChannel {
    PhaseA,
    PhaseB,
    PhaseC,
}

fn enable_clocks() {
    modify(RCC_AHB2ENR, |value| {
        value | RCC_AHB2ENR_GPIOBEN | RCC_AHB2ENR_ADC345EN
    });
    modify(RCC_APB2ENR, |value| value | RCC_APB2ENR_SYSCFGEN);
    let _ = read(RCC_AHB2ENR);
    let _ = read(RCC_APB2ENR);
}

fn configure_current_opamps() {
    modify(GPIO_MODER, |value| {
        (value & !CURRENT_PIN_MODE_MASK) | CURRENT_PIN_ANALOG_VALUE
    });
    modify(GPIO_OSPEEDR, |value| value & !CURRENT_PIN_MODE_MASK);
    modify(GPIO_AFRH, |value| value & !CURRENT_PIN_AFRH_MASK);

    write(OPAMP3_BASE, opamp_csr(/*vpsel=*/ 1));
    write(OPAMP4_BASE, opamp_csr(/*vpsel=*/ 2));
    write(OPAMP6_BASE, opamp_csr(/*vpsel=*/ 0));
}

fn configure_adc_common() {
    write(ADC_COMMON_CCR, ADC_CCR_VREFEN | ADC_CCR_CKMODE_HCLK_DIV4);
}

fn configure_current_adc(adc_base: usize, channel: CurrentAdcChannel) {
    match channel {
        CurrentAdcChannel::PhaseA => {
            write(
                adc_register(adc_base, ADC_SMPR2),
                sample_time(/*channel=*/ 13),
            );
            write(
                adc_register(adc_base, ADC_JSQR),
                injected_sequence(/*channel=*/ 13),
            );
        }
        CurrentAdcChannel::PhaseB => {
            write(
                adc_register(adc_base, ADC_SMPR2),
                sample_time(/*channel=*/ 17),
            );
            write(
                adc_register(adc_base, ADC_JSQR),
                injected_sequence(/*channel=*/ 17),
            );
        }
        CurrentAdcChannel::PhaseC => {
            write(
                adc_register(adc_base, ADC_SMPR1),
                sample_time(/*channel=*/ 5),
            );
            write(
                adc_register(adc_base, ADC_JSQR),
                injected_sequence(/*channel=*/ 5),
            );
        }
    }
}

fn enable_adc(adc_base: usize) -> Result<(), CurrentAdcError> {
    write(adc_register(adc_base, ADC_CR), ADC_CR_ADVREGEN);
    short_delay();
    modify(adc_register(adc_base, ADC_CR), |value| value | ADC_CR_ADCAL);
    wait_until_clear(adc_register(adc_base, ADC_CR), ADC_CR_ADCAL)
        .map_err(|()| CurrentAdcError::CalibrationTimeout { adc_base })?;
    short_delay();
    modify(adc_register(adc_base, ADC_CR), |value| {
        value | ADC_CR_ADCALDIF | ADC_CR_ADCAL
    });
    wait_until_clear(adc_register(adc_base, ADC_CR), ADC_CR_ADCAL)
        .map_err(|()| CurrentAdcError::CalibrationTimeout { adc_base })?;
    short_delay();

    write(adc_register(adc_base, ADC_ISR), ADC_ISR_ADRDY);
    modify(adc_register(adc_base, ADC_CR), |value| value | ADC_CR_ADEN);
    wait_until_set(adc_register(adc_base, ADC_ISR), ADC_ISR_ADRDY)
        .map_err(|()| CurrentAdcError::ReadyTimeout { adc_base })
}

fn start_injected_conversions() {
    modify(adc_register(ADC5_BASE, ADC_CR), |value| {
        value | ADC_CR_JADSTART
    });
    modify(adc_register(ADC4_BASE, ADC_CR), |value| {
        value | ADC_CR_JADSTART
    });
    modify(adc_register(ADC3_BASE, ADC_CR), |value| {
        value | ADC_CR_JADSTART
    });
}

fn read_jdr1(adc_base: usize) -> u16 {
    (read(adc_register(adc_base, ADC_JDR1)) & 0xFFFF) as u16
}

fn opamp_csr(vpsel: u32) -> u32 {
    (vpsel << OPAMP_CSR_VPSEL_POS)
        | (3 << OPAMP_CSR_VMSEL_POS)
        | OPAMP_CSR_HIGHSPEEDEN
        | OPAMP_CSR_OPAMPINTEN
        | OPAMP_CSR_ENABLE
}

fn injected_sequence(channel: u32) -> u32 {
    ADC_JSQR_HRTIM_TRG1 | (channel << ADC_JSQR_JSQ1_POS)
}

fn sample_time(channel: u32) -> u32 {
    ADC_SAMPLE_12_5_CYCLES << ((channel % 10) * 3)
}

fn wait_until_set(address: usize, mask: u32) -> Result<(), ()> {
    wait_for(address, mask, mask)
}

fn wait_until_clear(address: usize, mask: u32) -> Result<(), ()> {
    wait_for(address, mask, 0)
}

fn wait_for(address: usize, mask: u32, expected: u32) -> Result<(), ()> {
    for _ in 0..WAIT_ITERATIONS {
        if read(address) & mask == expected {
            return Ok(());
        }
        spin_loop();
    }
    Err(())
}

fn short_delay() {
    for _ in 0..2048 {
        spin_loop();
    }
}

const fn two_bit_pin_mask(pin: u32) -> u32 {
    0b11 << (pin * 2)
}

const fn four_bit_pin_mask(pin: u32) -> u32 {
    0b1111 << ((pin - 8) * 4)
}

fn adc_register(adc_base: usize, offset: usize) -> usize {
    adc_base + offset
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
