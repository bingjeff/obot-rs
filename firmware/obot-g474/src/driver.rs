use core::ptr::{read_volatile, write_volatile};

const RCC_BASE: usize = 0x4002_1000;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;

const GPIOC_BASE: usize = 0x4800_0800;
const GPIO_MODER: usize = GPIOC_BASE;
const GPIO_OSPEEDR: usize = GPIOC_BASE + 0x08;
const GPIO_PUPDR: usize = GPIOC_BASE + 0x0C;
const GPIO_IDR: usize = GPIOC_BASE + 0x10;
const GPIO_BSRR: usize = GPIOC_BASE + 0x18;

const DRIVER_ENABLE_PIN: u32 = 13;
const DRIVER_FAULT_PIN: u32 = 14;
const DRIVER_ENABLE_BIT: u32 = 1 << DRIVER_ENABLE_PIN;
const DRIVER_FAULT_BIT: u32 = 1 << DRIVER_FAULT_PIN;

const GPIO_MODE_INPUT: u32 = 0b00;
const GPIO_MODE_OUTPUT: u32 = 0b01;
const GPIO_SPEED_LOW: u32 = 0b00;
const GPIO_PULL_NONE: u32 = 0b00;
const GPIO_PULL_UP: u32 = 0b01;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DriverPinStatus {
    pub enabled: bool,
    pub faulted: bool,
}

pub struct MotorDriverPins;

impl MotorDriverPins {
    pub fn init_motor_hall_disabled() -> Self {
        enable_gpioc_clock();
        configure_enable_pin();
        configure_fault_pin();
        let pins = Self;
        pins.disable();
        pins
    }

    pub fn disable(&self) {
        write(GPIO_BSRR, DRIVER_ENABLE_BIT << 16);
    }

    pub fn enable(&self) {
        write(GPIO_BSRR, DRIVER_ENABLE_BIT);
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        read(GPIO_IDR) & DRIVER_ENABLE_BIT != 0
    }

    #[inline(always)]
    pub fn is_faulted(&self) -> bool {
        read(GPIO_IDR) & DRIVER_FAULT_BIT == 0
    }

    #[inline(always)]
    pub fn status(&self) -> DriverPinStatus {
        DriverPinStatus {
            enabled: self.is_enabled(),
            faulted: self.is_faulted(),
        }
    }
}

fn enable_gpioc_clock() {
    modify(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOCEN);
    let _ = read(RCC_AHB2ENR);
}

fn configure_enable_pin() {
    modify(GPIO_MODER, |value| {
        set_two_bit_field(value, DRIVER_ENABLE_PIN, GPIO_MODE_OUTPUT)
    });
    modify(GPIO_OSPEEDR, |value| {
        set_two_bit_field(value, DRIVER_ENABLE_PIN, GPIO_SPEED_LOW)
    });
    modify(GPIO_PUPDR, |value| {
        set_two_bit_field(value, DRIVER_ENABLE_PIN, GPIO_PULL_NONE)
    });
}

fn configure_fault_pin() {
    modify(GPIO_MODER, |value| {
        set_two_bit_field(value, DRIVER_FAULT_PIN, GPIO_MODE_INPUT)
    });
    modify(GPIO_OSPEEDR, |value| {
        set_two_bit_field(value, DRIVER_FAULT_PIN, GPIO_SPEED_LOW)
    });
    modify(GPIO_PUPDR, |value| {
        set_two_bit_field(value, DRIVER_FAULT_PIN, GPIO_PULL_UP)
    });
}

fn set_two_bit_field(value: u32, pin: u32, field: u32) -> u32 {
    let shift = pin * 2;
    (value & !(0b11 << shift)) | (field << shift)
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
