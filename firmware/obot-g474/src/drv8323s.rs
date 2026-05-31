use core::ptr::{read_volatile, write_volatile};

const RCC_BASE: usize = 0x4002_1000;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_APB2ENR: usize = RCC_BASE + 0x60;
const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_APB2ENR_SPI1EN: u32 = 1 << 12;

const GPIOA_BASE: usize = 0x4800_0000;
const GPIO_MODER: usize = GPIOA_BASE;
const GPIO_OSPEEDR: usize = GPIOA_BASE + 0x08;
const GPIO_PUPDR: usize = GPIOA_BASE + 0x0C;
const GPIO_BSRR: usize = GPIOA_BASE + 0x18;
const GPIO_AFRL: usize = GPIOA_BASE + 0x20;

const SPI1_BASE: usize = 0x4001_3000;
const SPI_CR1: usize = SPI1_BASE;
const SPI_CR2: usize = SPI1_BASE + 0x04;
const SPI_SR: usize = SPI1_BASE + 0x08;
const SPI_DR: usize = SPI1_BASE + 0x0C;

const GPIO_MODE_OUTPUT: u32 = 0b01;
const GPIO_MODE_ALT: u32 = 0b10;
const GPIO_SPEED_VERY_HIGH: u32 = 0b11;
const GPIO_PULL_NONE: u32 = 0b00;
const GPIO_PULL_UP: u32 = 0b01;
const GPIO_AF5: u32 = 5;

const SPI_CR1_MSTR: u32 = 1 << 2;
const SPI_CR1_BR_DIV64: u32 = 6 << 3;
const SPI_CR1_SPE: u32 = 1 << 6;
const SPI_CR2_FRF: u32 = 1 << 4;
const SPI_CR2_DS_16BIT: u32 = 15 << 8;
const SPI_SR_RXNE: u32 = 1 << 0;

const NSS_PIN: u32 = 4;
const SCK_PIN: u32 = 5;
const MISO_PIN: u32 = 6;
const MOSI_PIN: u32 = 7;
const STATUS_POLL_TIMEOUT: u32 = 16_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Drv8323sStatus {
    pub fault_status_1: u16,
    pub vgs_status_2: u16,
}

impl Drv8323sStatus {
    pub const fn as_u32(self) -> u32 {
        self.fault_status_1 as u32 | ((self.vgs_status_2 as u32) << 16)
    }
}

pub struct Drv8323s;

impl Drv8323s {
    #[cold]
    #[inline(never)]
    pub fn init_motor_hall() -> Self {
        enable_clocks();
        configure_spi_pins_idle();
        Self
    }

    #[cold]
    #[inline(never)]
    pub fn read_status(&self) -> Option<Drv8323sStatus> {
        self.start_transaction();
        let fault_status_1 = self.read_register(0);
        let vgs_status_2 = self.read_register(1);
        self.end_transaction();

        Some(Drv8323sStatus {
            fault_status_1: fault_status_1?,
            vgs_status_2: vgs_status_2?,
        })
    }

    fn start_transaction(&self) {
        configure_nss_alternate();
        write(SPI_CR1, 0);
        write(SPI_CR2, SPI_CR2_DS_16BIT | SPI_CR2_FRF);
        write(SPI_CR1, SPI_CR1_MSTR | SPI_CR1_BR_DIV64 | SPI_CR1_SPE);
    }

    fn end_transaction(&self) {
        write(SPI_CR1, 0);
        configure_nss_idle_high();
    }

    fn read_register(&self, address: u8) -> Option<u16> {
        let command = (1_u16 << 15) | (((address as u16) & 0x7) << 11);
        transfer(command)
    }
}

fn enable_clocks() {
    modify(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOAEN);
    modify(RCC_APB2ENR, |value| value | RCC_APB2ENR_SPI1EN);
    let _ = read(RCC_AHB2ENR);
    let _ = read(RCC_APB2ENR);
}

fn configure_spi_pins_idle() {
    configure_pin_alt(SCK_PIN, GPIO_PULL_NONE);
    configure_pin_alt(MISO_PIN, GPIO_PULL_UP);
    configure_pin_alt(MOSI_PIN, GPIO_PULL_NONE);
    configure_nss_idle_high();
}

fn configure_nss_alternate() {
    configure_pin_alt(NSS_PIN, GPIO_PULL_NONE);
}

fn configure_nss_idle_high() {
    write(GPIO_BSRR, 1 << NSS_PIN);
    modify(GPIO_MODER, |value| {
        set_two_bit_field(value, NSS_PIN, GPIO_MODE_OUTPUT)
    });
    modify(GPIO_OSPEEDR, |value| {
        set_two_bit_field(value, NSS_PIN, GPIO_SPEED_VERY_HIGH)
    });
    modify(GPIO_PUPDR, |value| {
        set_two_bit_field(value, NSS_PIN, GPIO_PULL_NONE)
    });
}

fn configure_pin_alt(pin: u32, pull: u32) {
    modify(GPIO_MODER, |value| {
        set_two_bit_field(value, pin, GPIO_MODE_ALT)
    });
    modify(GPIO_OSPEEDR, |value| {
        set_two_bit_field(value, pin, GPIO_SPEED_VERY_HIGH)
    });
    modify(GPIO_PUPDR, |value| set_two_bit_field(value, pin, pull));
    modify(GPIO_AFRL, |value| set_four_bit_field(value, pin, GPIO_AF5));
}

fn transfer(word: u16) -> Option<u16> {
    write16(SPI_DR, word);
    for _ in 0..STATUS_POLL_TIMEOUT {
        if read(SPI_SR) & SPI_SR_RXNE != 0 {
            return Some(read16(SPI_DR));
        }
    }
    None
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

fn read16(address: usize) -> u16 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    // Volatile access is required so register reads are not elided or cached.
    unsafe { read_volatile(address as *const u16) }
}

fn write16(address: usize, value: u16) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    // Volatile access is required so register writes are performed as requested.
    unsafe { write_volatile(address as *mut u16, value) };
}
