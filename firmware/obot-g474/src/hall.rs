use core::ptr::{read_volatile, write_volatile};

use obot_core::hall::{HallEncoder, HallSample};

const RCC_BASE: usize = 0x4002_1000;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;

const GPIOA_BASE: usize = 0x4800_0000;
const GPIO_MODER: usize = GPIOA_BASE;
const GPIO_OSPEEDR: usize = GPIOA_BASE + 0x08;
const GPIO_PUPDR: usize = GPIOA_BASE + 0x0C;
const GPIO_IDR: usize = GPIOA_BASE + 0x10;
const GPIO_AFRL: usize = GPIOA_BASE + 0x20;

const HALL_PIN_MASK: u32 = 0b111;
const HALL_MODE_MASK: u32 = two_bit_pin_mask(0) | two_bit_pin_mask(1) | two_bit_pin_mask(2);
const HALL_AFRL_MASK: u32 = four_bit_pin_mask(0) | four_bit_pin_mask(1) | four_bit_pin_mask(2);
const GPIO_SPEED_VERY_HIGH: u32 = 0b11;
const HALL_SPEED_VALUE: u32 =
    GPIO_SPEED_VERY_HIGH | (GPIO_SPEED_VERY_HIGH << 2) | (GPIO_SPEED_VERY_HIGH << 4);

pub struct HallInputs {
    encoder: HallEncoder,
}

impl HallInputs {
    pub fn init_motor_hall() -> Self {
        enable_gpioa_clock();
        configure_hall_pins();
        Self {
            encoder: HallEncoder::new(),
        }
    }

    pub fn read_count(&mut self) -> i32 {
        self.read_sample().count
    }

    pub fn read_sample(&mut self) -> HallSample {
        self.encoder.read_sample(self.raw_bits())
    }

    pub fn raw_bits(&self) -> u8 {
        (read(GPIO_IDR) & HALL_PIN_MASK) as u8
    }
}

fn enable_gpioa_clock() {
    modify(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOAEN);
    let _ = read(RCC_AHB2ENR);
}

fn configure_hall_pins() {
    modify(GPIO_MODER, |value| value & !HALL_MODE_MASK);
    modify(GPIO_OSPEEDR, |value| {
        (value & !HALL_MODE_MASK) | HALL_SPEED_VALUE
    });
    modify(GPIO_PUPDR, |value| value & !HALL_MODE_MASK);
    modify(GPIO_AFRL, |value| value & !HALL_AFRL_MASK);
}

const fn two_bit_pin_mask(pin: u32) -> u32 {
    0b11 << (pin * 2)
}

const fn four_bit_pin_mask(pin: u32) -> u32 {
    0b1111 << (pin * 4)
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
