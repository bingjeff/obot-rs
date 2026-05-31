use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU32, Ordering};

const RCC_BASE: usize = 0x4002_1000;
const RCC_APB1RSTR1: usize = RCC_BASE + 0x38;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_APB1ENR1: usize = RCC_BASE + 0x58;

const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_APB1RSTR1_USBRST: u32 = 1 << 23;
const RCC_APB1ENR1_USBEN: u32 = 1 << 23;

const GPIOA_BASE: usize = 0x4800_0000;
const GPIO_MODER: usize = GPIOA_BASE;
const GPIO_OSPEEDR: usize = GPIOA_BASE + 0x08;
const GPIO_PUPDR: usize = GPIOA_BASE + 0x0C;

const USB_BASE: usize = 0x4000_5C00;
const USB_CNTR: usize = USB_BASE + 0x40;
const USB_ISTR: usize = USB_BASE + 0x44;
const USB_DADDR: usize = USB_BASE + 0x4C;
const USB_BTABLE: usize = USB_BASE + 0x50;
const USB_BCDR: usize = USB_BASE + 0x58;

const USB_DM_PIN: u32 = 11;
const USB_DP_PIN: u32 = 12;
const USB_PIN_MASK: u32 = two_bit_pin_mask(USB_DM_PIN) | two_bit_pin_mask(USB_DP_PIN);

const GPIO_MODE_ANALOG: u32 = 0b11;
const GPIO_PULL_NONE: u32 = 0b00;

const USB_CNTR_L1REQM: u16 = 0x0080;
const USB_CNTR_RESETM: u16 = 0x0400;
const USB_CNTR_SUSPM: u16 = 0x0800;
const USB_CNTR_WKUPM: u16 = 0x1000;
const USB_CNTR_ERRM: u16 = 0x2000;
const USB_CNTR_CTRM: u16 = 0x8000;
const USB_CNTR_INITIAL_MASKS: u16 = USB_CNTR_L1REQM
    | USB_CNTR_RESETM
    | USB_CNTR_SUSPM
    | USB_CNTR_WKUPM
    | USB_CNTR_ERRM
    | USB_CNTR_CTRM;

const USB_ISTR_CLEAR_ALL: u16 = 0;
const USB_BCDR_DPPU: u16 = 0x8000;

static USB_LP_INTERRUPT_COUNT: AtomicU32 = AtomicU32::new(0);

pub struct UsbDevice;

impl UsbDevice {
    pub fn prepare_disconnected() -> Self {
        enable_gpioa_clock();
        configure_usb_pins();
        enable_usb_clock();
        reset_usb();
        configure_usb_disconnected();
        Self
    }
}

pub fn interrupt() {
    USB_LP_INTERRUPT_COUNT.fetch_add(1, Ordering::Relaxed);
    write16(USB_ISTR, USB_ISTR_CLEAR_ALL);
}

pub fn interrupt_count() -> u32 {
    USB_LP_INTERRUPT_COUNT.load(Ordering::Relaxed)
}

fn enable_gpioa_clock() {
    modify32(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOAEN);
    let _ = read32(RCC_AHB2ENR);
}

fn configure_usb_pins() {
    modify32(GPIO_MODER, |value| {
        set_two_bit_field(
            set_two_bit_field(value, USB_DM_PIN, GPIO_MODE_ANALOG),
            USB_DP_PIN,
            GPIO_MODE_ANALOG,
        )
    });
    modify32(GPIO_OSPEEDR, |value| value & !USB_PIN_MASK);
    modify32(GPIO_PUPDR, |value| {
        set_two_bit_field(
            set_two_bit_field(value, USB_DM_PIN, GPIO_PULL_NONE),
            USB_DP_PIN,
            GPIO_PULL_NONE,
        )
    });
}

fn enable_usb_clock() {
    modify32(RCC_APB1ENR1, |value| value | RCC_APB1ENR1_USBEN);
    let _ = read32(RCC_APB1ENR1);
}

fn reset_usb() {
    modify32(RCC_APB1RSTR1, |value| value | RCC_APB1RSTR1_USBRST);
    modify32(RCC_APB1RSTR1, |value| value & !RCC_APB1RSTR1_USBRST);
}

fn configure_usb_disconnected() {
    modify16(USB_BCDR, |value| value & !USB_BCDR_DPPU);
    write16(USB_BTABLE, 0);
    write16(USB_DADDR, 0);
    write16(USB_ISTR, USB_ISTR_CLEAR_ALL);
    write16(USB_CNTR, USB_CNTR_INITIAL_MASKS);
}

fn set_two_bit_field(value: u32, pin: u32, field: u32) -> u32 {
    let shift = pin * 2;
    (value & !(0b11 << shift)) | (field << shift)
}

const fn two_bit_pin_mask(pin: u32) -> u32 {
    0b11 << (pin * 2)
}

fn modify32(address: usize, f: impl FnOnce(u32) -> u32) {
    let value = read32(address);
    write32(address, f(value));
}

fn modify16(address: usize, f: impl FnOnce(u16) -> u16) {
    let value = read16(address);
    write16(address, f(value));
}

fn read32(address: usize) -> u32 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { read_volatile(address as *const u32) }
}

fn write32(address: usize, value: u32) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u32, value) };
}

fn read16(address: usize) -> u16 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { read_volatile(address as *const u16) }
}

fn write16(address: usize, value: u16) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u16, value) };
}
