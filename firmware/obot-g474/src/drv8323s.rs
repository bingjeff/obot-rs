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
const SPI_CR2_TI_FRAME_FORMAT: u32 = 1 << 4;
const SPI_CR2_DS_16BIT: u32 = 15 << 8;
const SPI_SR_RXNE: u32 = 1 << 0;

const NSS_PIN: u32 = 4;
const SCK_PIN: u32 = 5;
const MISO_PIN: u32 = 6;
const MOSI_PIN: u32 = 7;
const STATUS_POLL_TIMEOUT: u32 = 16_000;
const TRANSFER_STATUS_BEFORE_BIT: u16 = 1 << 0;
const TRANSFER_CONFIG_SHIFT: u16 = 1;
const TRANSFER_STATUS_AFTER_BIT: u16 = 1 << 6;

pub const MOTOR_HALL_REGS: [u16; 5] = [
    2 << 11,
    (3 << 11) | 0x3FF,
    (4 << 11) | 0x37F,
    5 << 11,
    (6 << 11) | 0x2C0,
];

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Drv8323sConfigReport {
    pub status_before: Option<Drv8323sStatus>,
    pub status_after: Option<Drv8323sStatus>,
    pub verify_error_mask: u16,
    pub transfer_error_mask: u16,
}

impl Drv8323sConfigReport {
    pub const fn configured(self) -> bool {
        self.verify_error_mask == 0 && self.transfer_error_mask == 0
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
    pub fn configure_motor_hall_registers(&self) -> Drv8323sConfigReport {
        self.configure_registers(&MOTOR_HALL_REGS)
    }

    #[cold]
    #[inline(never)]
    pub fn read_status(&self) -> Option<Drv8323sStatus> {
        self.start_transaction();
        let status = self.read_status_in_transaction();
        self.end_transaction();
        status
    }

    fn configure_registers(&self, registers: &[u16]) -> Drv8323sConfigReport {
        self.start_transaction();

        let mut report = Drv8323sConfigReport {
            status_before: self.read_status_in_transaction(),
            ..Drv8323sConfigReport::default()
        };
        if status_is_no_response(report.status_before) {
            report.transfer_error_mask |= TRANSFER_STATUS_BEFORE_BIT;
        }

        for (index, reg_out) in registers.iter().copied().enumerate() {
            let transfer_bit = 1 << (TRANSFER_CONFIG_SHIFT + index as u16);
            let address = ((reg_out >> 11) & 0x7) as u8;
            let write_ok = self.write_register(reg_out).is_some();
            let reg_in = self.read_register(address);
            match (write_ok, reg_in) {
                (_, Some(reg_in)) if register_is_no_response(reg_in) => {
                    report.transfer_error_mask |= transfer_bit
                }
                (true, Some(reg_in)) if (reg_in & 0x07FF) == (reg_out & 0x07FF) => {}
                (true, Some(_)) => report.verify_error_mask |= 1 << index,
                _ => report.transfer_error_mask |= transfer_bit,
            }
        }

        report.status_after = self.read_status_in_transaction();
        if status_is_no_response(report.status_after) {
            report.transfer_error_mask |= TRANSFER_STATUS_AFTER_BIT;
        }

        self.end_transaction();
        report
    }

    fn start_transaction(&self) {
        configure_nss_alt();
        write(SPI_CR1, 0);
        write(SPI_CR2, SPI_CR2_DS_16BIT | SPI_CR2_TI_FRAME_FORMAT);
        write(SPI_CR1, SPI_CR1_MSTR | SPI_CR1_BR_DIV64 | SPI_CR1_SPE);
    }

    fn end_transaction(&self) {
        write(GPIO_BSRR, 1 << NSS_PIN);
        write(SPI_CR1, 0);
        configure_nss_idle_high();
    }

    fn read_status_in_transaction(&self) -> Option<Drv8323sStatus> {
        let fault_status_1 = self.read_register(0);
        let vgs_status_2 = self.read_register(1);

        Some(Drv8323sStatus {
            fault_status_1: fault_status_1?,
            vgs_status_2: vgs_status_2?,
        })
    }

    fn write_register(&self, word: u16) -> Option<u16> {
        transfer(word)
    }

    fn read_register(&self, address: u8) -> Option<u16> {
        let command = (1_u16 << 15) | (((address as u16) & 0x7) << 11);
        transfer(command)
    }
}

fn status_is_no_response(status: Option<Drv8323sStatus>) -> bool {
    match status {
        Some(status) => {
            register_is_no_response(status.fault_status_1)
                && register_is_no_response(status.vgs_status_2)
        }
        None => true,
    }
}

const fn register_is_no_response(value: u16) -> bool {
    value == 0xFFFF
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

fn configure_nss_alt() {
    configure_pin_alt(NSS_PIN, GPIO_PULL_NONE);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ones_status_is_no_response() {
        assert!(status_is_no_response(Some(Drv8323sStatus {
            fault_status_1: 0xFFFF,
            vgs_status_2: 0xFFFF,
        })));
        assert!(status_is_no_response(None));
    }

    #[test]
    fn mixed_status_is_reportable_response() {
        assert!(!status_is_no_response(Some(Drv8323sStatus {
            fault_status_1: 0x0000,
            vgs_status_2: 0xFFFF,
        })));
    }
}
