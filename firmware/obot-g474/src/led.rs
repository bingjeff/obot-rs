use core::ptr::{read_volatile, write_volatile};

const RCC_BASE: usize = 0x4002_1000;
const RCC_APB1ENR1: usize = RCC_BASE + 0x58;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_APB1ENR1_TIM4EN: u32 = 1 << 2;
const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;

const GPIOB_BASE: usize = 0x4800_0400;
const GPIO_MODER: usize = GPIOB_BASE;
const GPIO_OSPEEDR: usize = GPIOB_BASE + 0x08;
const GPIO_AFRL: usize = GPIOB_BASE + 0x20;
const GPIO_AFRH: usize = GPIOB_BASE + 0x24;

const TIM4_BASE: usize = 0x4000_0800;
const TIM_CR1: usize = TIM4_BASE;
const TIM_ARR: usize = TIM4_BASE + 0x2C;
const TIM_CCMR1: usize = TIM4_BASE + 0x18;
const TIM_CCMR2: usize = TIM4_BASE + 0x1C;
const TIM_CCER: usize = TIM4_BASE + 0x20;
const TIM_CCR1: usize = TIM4_BASE + 0x34;
const TIM_CCR2: usize = TIM4_BASE + 0x38;
const TIM_CCR3: usize = TIM4_BASE + 0x3C;

const LED_RED_PIN: u32 = 6;
const LED_GREEN_PIN: u32 = 7;
const LED_BLUE_PIN: u32 = 8;
const GPIO_MODE_ALT: u32 = 0b10;
const GPIO_SPEED_VERY_HIGH: u32 = 0b11;
const GPIO_AF2_TIM4: u32 = 2;

const TIM_CCMR_OC1PE: u32 = 1 << 3;
const TIM_CCMR_OC2PE: u32 = 1 << 11;
const TIM_CCMR_OC3PE: u32 = 1 << 3;
const TIM_CCMR_OC1M_PWM_MODE_1: u32 = 0b110 << 4;
const TIM_CCMR_OC2M_PWM_MODE_1: u32 = 0b110 << 12;
const TIM_CCMR_OC3M_PWM_MODE_1: u32 = 0b110 << 4;
const TIM_CCER_CC1E: u32 = 1 << 0;
const TIM_CCER_CC2E: u32 = 1 << 4;
const TIM_CCER_CC3E: u32 = 1 << 8;
const TIM_CR1_CEN: u32 = 1 << 0;
const LED_PERIOD_COUNTS: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedChannelMap {
    pub red: LedChannel,
    pub green: LedChannel,
    pub blue: LedChannel,
}

impl LedChannelMap {
    pub const MOTOR_HALL_DEFAULT: Self = Self {
        red: LedChannel::Ch1,
        green: LedChannel::Ch2,
        blue: LedChannel::Ch3,
    };

    pub const MOTOR_HALL_R0_R4_MR0P: Self = Self {
        red: LedChannel::Ch1,
        green: LedChannel::Ch3,
        blue: LedChannel::Ch2,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedChannel {
    Ch1,
    Ch2,
    Ch3,
}

pub struct StatusLed {
    channels: LedChannelMap,
}

impl StatusLed {
    pub fn init_motor_hall() -> Self {
        Self::init_with_channels(LedChannelMap::MOTOR_HALL_DEFAULT)
    }

    pub fn init_motor_hall_r0_r4_mr0p() -> Self {
        Self::init_with_channels(LedChannelMap::MOTOR_HALL_R0_R4_MR0P)
    }

    pub fn init_with_channels(channels: LedChannelMap) -> Self {
        enable_clocks();
        configure_led_pins();
        configure_tim4_pwm();
        let led = Self { channels };
        led.set_rgb_raw(0, 0, 0);
        led
    }

    pub fn set_rgb_raw(&self, red: u16, green: u16, blue: u16) {
        self.write_channel(self.channels.red, red);
        self.write_channel(self.channels.green, green);
        self.write_channel(self.channels.blue, blue);
    }

    pub const fn channels(&self) -> LedChannelMap {
        self.channels
    }

    fn write_channel(&self, channel: LedChannel, value: u16) {
        write(channel_compare_register(channel), value as u32);
    }
}

fn enable_clocks() {
    modify(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOBEN);
    modify(RCC_APB1ENR1, |value| value | RCC_APB1ENR1_TIM4EN);
    let _ = read(RCC_AHB2ENR);
    let _ = read(RCC_APB1ENR1);
}

fn configure_led_pins() {
    configure_pin(
        LED_RED_PIN,
        GPIO_MODE_ALT,
        GPIO_SPEED_VERY_HIGH,
        GPIO_AF2_TIM4,
    );
    configure_pin(
        LED_GREEN_PIN,
        GPIO_MODE_ALT,
        GPIO_SPEED_VERY_HIGH,
        GPIO_AF2_TIM4,
    );
    configure_pin(
        LED_BLUE_PIN,
        GPIO_MODE_ALT,
        GPIO_SPEED_VERY_HIGH,
        GPIO_AF2_TIM4,
    );
}

fn configure_tim4_pwm() {
    write(TIM_CCR1, 0);
    write(TIM_CCR2, 0);
    write(TIM_CCR3, 0);
    write(TIM_ARR, LED_PERIOD_COUNTS as u32);
    write(
        TIM_CCMR1,
        TIM_CCMR_OC1PE | TIM_CCMR_OC1M_PWM_MODE_1 | TIM_CCMR_OC2PE | TIM_CCMR_OC2M_PWM_MODE_1,
    );
    write(TIM_CCMR2, TIM_CCMR_OC3PE | TIM_CCMR_OC3M_PWM_MODE_1);
    write(TIM_CCER, TIM_CCER_CC1E | TIM_CCER_CC2E | TIM_CCER_CC3E);
    write(TIM_CR1, TIM_CR1_CEN);
}

fn configure_pin(pin: u32, mode: u32, speed: u32, alternate_function: u32) {
    modify(GPIO_MODER, |value| set_two_bit_field(value, pin, mode));
    modify(GPIO_OSPEEDR, |value| set_two_bit_field(value, pin, speed));
    let afr = if pin < 8 { GPIO_AFRL } else { GPIO_AFRH };
    let afr_pin = pin % 8;
    modify(afr, |value| {
        set_four_bit_field(value, afr_pin, alternate_function)
    });
}

const fn channel_compare_register(channel: LedChannel) -> usize {
    match channel {
        LedChannel::Ch1 => TIM_CCR1,
        LedChannel::Ch2 => TIM_CCR2,
        LedChannel::Ch3 => TIM_CCR3,
    }
}

fn set_two_bit_field(value: u32, pin: u32, field: u32) -> u32 {
    let shift = pin * 2;
    (value & !(0b11 << shift)) | (field << shift)
}

fn set_four_bit_field(value: u32, pin: u32, field: u32) -> u32 {
    let shift = pin * 4;
    (value & !(0b1111 << shift)) | (field << shift)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_hall_led_pins_match_cpp_setup() {
        assert_eq!(LED_RED_PIN, 6);
        assert_eq!(LED_GREEN_PIN, 7);
        assert_eq!(LED_BLUE_PIN, 8);
        assert_eq!(GPIO_AF2_TIM4, 2);
        assert_eq!(LED_PERIOD_COUNTS, 65_535);
    }

    #[test]
    fn motor_hall_channel_maps_match_cpp_macros() {
        assert_eq!(
            LedChannelMap::MOTOR_HALL_DEFAULT,
            LedChannelMap {
                red: LedChannel::Ch1,
                green: LedChannel::Ch2,
                blue: LedChannel::Ch3,
            }
        );
        assert_eq!(
            LedChannelMap::MOTOR_HALL_R0_R4_MR0P,
            LedChannelMap {
                red: LedChannel::Ch1,
                green: LedChannel::Ch3,
                blue: LedChannel::Ch2,
            }
        );
    }

    #[test]
    fn channel_registers_are_tim4_ccr1_to_ccr3() {
        assert_eq!(channel_compare_register(LedChannel::Ch1), TIM_CCR1);
        assert_eq!(channel_compare_register(LedChannel::Ch2), TIM_CCR2);
        assert_eq!(channel_compare_register(LedChannel::Ch3), TIM_CCR3);
    }
}
