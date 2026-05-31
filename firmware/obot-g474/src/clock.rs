use core::{
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
};

pub const CPU_HZ: u32 = 170_000_000;

const RCC_BASE: usize = 0x4002_1000;
const RCC_CR: usize = RCC_BASE;
const RCC_CFGR: usize = RCC_BASE + 0x08;
const RCC_PLLCFGR: usize = RCC_BASE + 0x0C;
const RCC_APB1RSTR1: usize = RCC_BASE + 0x38;
const RCC_APB1ENR1: usize = RCC_BASE + 0x58;
const RCC_CCIPR: usize = RCC_BASE + 0x88;
const RCC_CRRCR: usize = RCC_BASE + 0x98;

const FLASH_BASE: usize = 0x4002_2000;
const FLASH_ACR: usize = FLASH_BASE;

const PWR_BASE: usize = 0x4000_7000;
const PWR_SR2: usize = PWR_BASE + 0x14;
const PWR_CR5: usize = PWR_BASE + 0x80;

const CRS_BASE: usize = 0x4000_2000;
const CRS_CR: usize = CRS_BASE;
const CRS_CFGR: usize = CRS_BASE + 0x04;

const RCC_CR_HSION: u32 = 1 << 8;
const RCC_CR_HSIRDY: u32 = 1 << 10;
const RCC_CR_PLLON: u32 = 1 << 24;
const RCC_CR_PLLRDY: u32 = 1 << 25;

const RCC_CRRCR_HSI48ON: u32 = 1 << 0;
const RCC_CRRCR_HSI48RDY: u32 = 1 << 1;

const RCC_CFGR_SW: u32 = 0x3;
const RCC_CFGR_SW_HSI: u32 = 0x1;
const RCC_CFGR_SW_PLL: u32 = 0x3;
const RCC_CFGR_SWS: u32 = 0xC;
const RCC_CFGR_SWS_HSI: u32 = 0x4;
const RCC_CFGR_SWS_PLL: u32 = 0xC;
const RCC_CFGR_HPRE: u32 = 0xF0;
const RCC_CFGR_PPRE1: u32 = 0x0700;
const RCC_CFGR_PPRE2: u32 = 0x3800;

const RCC_PLLCFGR_PLLSRC_HSI: u32 = 0x2;
const RCC_PLLCFGR_PLLM_DIV4: u32 = (4 - 1) << 4;
const RCC_PLLCFGR_PLLN_85: u32 = 85 << 8;
const RCC_PLLCFGR_PLLREN: u32 = 1 << 24;
const RCC_PLLCFGR_PLLR_DIV2: u32 = 0 << 25;
const RCC_PLLCFGR_170MHZ_HSI: u32 = RCC_PLLCFGR_PLLSRC_HSI
    | RCC_PLLCFGR_PLLM_DIV4
    | RCC_PLLCFGR_PLLN_85
    | RCC_PLLCFGR_PLLREN
    | RCC_PLLCFGR_PLLR_DIV2;

const RCC_APB1RSTR1_CRSRST: u32 = 1 << 8;
const RCC_APB1ENR1_CRSEN: u32 = 1 << 8;
const RCC_APB1ENR1_PWREN: u32 = 1 << 28;
const RCC_CCIPR_CLK48SEL: u32 = 0x3 << 26;
const RCC_CCIPR_CLK48SEL_HSI48: u32 = 0x0 << 26;
const FLASH_ACR_LATENCY: u32 = 0xF;
const FLASH_ACR_LATENCY_8WS: u32 = 0x8;
const FLASH_ACR_PRFTEN: u32 = 1 << 8;
const FLASH_ACR_ICEN: u32 = 1 << 9;
const FLASH_ACR_DCEN: u32 = 1 << 10;
const FLASH_ACR_ACCELERATION: u32 = FLASH_ACR_PRFTEN | FLASH_ACR_ICEN | FLASH_ACR_DCEN;
const PWR_CR5_R1MODE: u32 = 1 << 8;
const PWR_SR2_VOSF: u32 = 1 << 10;

const CRS_CR_CEN: u32 = 1 << 5;
const CRS_CR_AUTOTRIMEN: u32 = 1 << 6;
const CRS_CR_TRIM_32: u32 = 32 << 8;
const CRS_CFGR_RELOAD_48MHZ_1KHZ: u32 = 47_999;
const CRS_CFGR_FELIM_34: u32 = 34 << 16;
const CRS_CFGR_SYNCSRC_USB_SOF: u32 = 0x2 << 28;
const CRS_CFGR_USB_SOF_SYNC: u32 =
    CRS_CFGR_RELOAD_48MHZ_1KHZ | CRS_CFGR_FELIM_34 | CRS_CFGR_SYNCSRC_USB_SOF;

const WAIT_ITERATIONS: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockError {
    HsiNotReady,
    VoltageScaleNotReady,
    HsiSwitchFailed,
    PllDisableFailed,
    PllNotReady,
    PllSwitchFailed,
    Hsi48NotReady,
}

pub fn configure_170mhz_hsi() -> Result<(), ClockError> {
    enable_hsi()?;
    configure_power_boost()?;
    set_flash_latency_8ws();
    switch_system_clock(
        RCC_CFGR_SW_HSI,
        RCC_CFGR_SWS_HSI,
        ClockError::HsiSwitchFailed,
    )?;
    disable_pll()?;
    write(RCC_PLLCFGR, RCC_PLLCFGR_170MHZ_HSI);
    enable_pll()?;
    switch_to_pll()
}

pub fn configure_usb_hsi48_crs() -> Result<(), ClockError> {
    enable_hsi48()?;
    select_usb_clock_hsi48();
    enable_crs_clock();
    reset_crs();
    configure_crs_usb_sof_sync();
    Ok(())
}

fn enable_hsi() -> Result<(), ClockError> {
    modify(RCC_CR, |value| value | RCC_CR_HSION);
    wait_until_set(RCC_CR, RCC_CR_HSIRDY).map_err(|()| ClockError::HsiNotReady)
}

fn enable_hsi48() -> Result<(), ClockError> {
    modify(RCC_CRRCR, |value| value | RCC_CRRCR_HSI48ON);
    wait_until_set(RCC_CRRCR, RCC_CRRCR_HSI48RDY).map_err(|()| ClockError::Hsi48NotReady)
}

fn select_usb_clock_hsi48() {
    modify(RCC_CCIPR, |value| {
        (value & !RCC_CCIPR_CLK48SEL) | RCC_CCIPR_CLK48SEL_HSI48
    });
}

fn enable_crs_clock() {
    modify(RCC_APB1ENR1, |value| value | RCC_APB1ENR1_CRSEN);
    let _ = read(RCC_APB1ENR1);
}

fn reset_crs() {
    modify(RCC_APB1RSTR1, |value| value | RCC_APB1RSTR1_CRSRST);
    modify(RCC_APB1RSTR1, |value| value & !RCC_APB1RSTR1_CRSRST);
}

fn configure_crs_usb_sof_sync() {
    modify(CRS_CR, |value| value & !(CRS_CR_CEN | CRS_CR_AUTOTRIMEN));
    write(CRS_CFGR, CRS_CFGR_USB_SOF_SYNC);
    write(CRS_CR, CRS_CR_TRIM_32 | CRS_CR_AUTOTRIMEN | CRS_CR_CEN);
}

fn configure_power_boost() -> Result<(), ClockError> {
    modify(RCC_APB1ENR1, |value| value | RCC_APB1ENR1_PWREN);
    let _ = read(RCC_APB1ENR1);
    modify(PWR_CR5, |value| value & !PWR_CR5_R1MODE);
    wait_until_clear(PWR_SR2, PWR_SR2_VOSF).map_err(|()| ClockError::VoltageScaleNotReady)
}

fn set_flash_latency_8ws() {
    modify(FLASH_ACR, |value| {
        (value & !FLASH_ACR_LATENCY) | FLASH_ACR_LATENCY_8WS | FLASH_ACR_ACCELERATION
    });
}

fn disable_pll() -> Result<(), ClockError> {
    modify(RCC_CR, |value| value & !RCC_CR_PLLON);
    wait_until_clear(RCC_CR, RCC_CR_PLLRDY).map_err(|()| ClockError::PllDisableFailed)
}

fn enable_pll() -> Result<(), ClockError> {
    modify(RCC_CR, |value| value | RCC_CR_PLLON);
    wait_until_set(RCC_CR, RCC_CR_PLLRDY).map_err(|()| ClockError::PllNotReady)
}

fn switch_to_pll() -> Result<(), ClockError> {
    modify(RCC_CFGR, |value| {
        value & !(RCC_CFGR_HPRE | RCC_CFGR_PPRE1 | RCC_CFGR_PPRE2)
    });
    switch_system_clock(
        RCC_CFGR_SW_PLL,
        RCC_CFGR_SWS_PLL,
        ClockError::PllSwitchFailed,
    )
}

fn switch_system_clock(sw: u32, sws: u32, error: ClockError) -> Result<(), ClockError> {
    modify(RCC_CFGR, |value| (value & !RCC_CFGR_SW) | sw);
    wait_for(RCC_CFGR, RCC_CFGR_SWS, sws).map_err(|()| error)
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
