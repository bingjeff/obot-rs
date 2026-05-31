use core::ptr::{read_volatile, write_volatile};

pub trait CycleCounter {
    fn now(&self) -> u32;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DwtCycleCounter;

impl DwtCycleCounter {
    const DEMCR: *mut u32 = 0xE000_EDFC as *mut u32;
    const DWT_CTRL: *mut u32 = 0xE000_1000 as *mut u32;
    const DWT_CYCCNT: *mut u32 = 0xE000_1004 as *mut u32;
    const DWT_LAR: *mut u32 = 0xE000_1FB0 as *mut u32;
    const DEMCR_TRCENA: u32 = 1 << 24;
    const DWT_CTRL_CYCCNTENA: u32 = 1;
    const DWT_LAR_UNLOCK: u32 = 0xC5AC_CE55;

    pub const fn new() -> Self {
        Self
    }

    pub fn enable(self) {
        self.enable_trace();
        self.unlock();
        self.reset();
        self.enable_counter();
    }

    pub fn reset(self) {
        // SAFETY: `DWT_CYCCNT` is the ARM DWT cycle-count register on Cortex-M4.
        // Volatile access is required because this address is memory-mapped I/O.
        unsafe { write_volatile(Self::DWT_CYCCNT, 0) };
    }

    fn enable_trace(self) {
        // SAFETY: `DEMCR` is the ARM debug exception and monitor control register.
        // This read-modify-write only sets TRCENA and preserves existing bits.
        let demcr = unsafe { read_volatile(Self::DEMCR) };
        // SAFETY: See the read above; volatile write is required for MMIO.
        unsafe { write_volatile(Self::DEMCR, demcr | Self::DEMCR_TRCENA) };
    }

    fn unlock(self) {
        // SAFETY: `DWT_LAR` is the ARM DWT lock access register on cores that
        // implement it. Writing the unlock key is required on STM32G4 before
        // `CYCCNT` reliably advances after reset.
        unsafe { write_volatile(Self::DWT_LAR, Self::DWT_LAR_UNLOCK) };
    }

    fn enable_counter(self) {
        // SAFETY: `DWT_CTRL` is the ARM DWT control register. This read-modify-write
        // only enables CYCCNT and preserves existing bits.
        let ctrl = unsafe { read_volatile(Self::DWT_CTRL) };
        // SAFETY: See the read above; volatile write is required for MMIO.
        unsafe { write_volatile(Self::DWT_CTRL, ctrl | Self::DWT_CTRL_CYCCNTENA) };
    }
}

impl CycleCounter for DwtCycleCounter {
    fn now(&self) -> u32 {
        // SAFETY: `DWT_CYCCNT` is a read-only use of the ARM DWT cycle-count register.
        // Volatile access prevents the compiler from caching or removing the read.
        unsafe { read_volatile(Self::DWT_CYCCNT) }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManualCycleCounter {
    now_cycles: u32,
}

impl ManualCycleCounter {
    pub const fn new(now_cycles: u32) -> Self {
        Self { now_cycles }
    }

    pub fn set(&mut self, now_cycles: u32) {
        self.now_cycles = now_cycles;
    }

    pub fn advance(&mut self, cycles: u32) {
        self.now_cycles = self.now_cycles.wrapping_add(cycles);
    }
}

impl CycleCounter for ManualCycleCounter {
    fn now(&self) -> u32 {
        self.now_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_counter_reports_and_advances_time() {
        let mut counter = ManualCycleCounter::new(10);

        assert_eq!(counter.now(), 10);
        counter.advance(5);
        assert_eq!(counter.now(), 15);
        counter.set(42);
        assert_eq!(counter.now(), 42);
    }

    #[test]
    fn manual_counter_wraps_like_the_dwt_counter() {
        let mut counter = ManualCycleCounter::new(u32::MAX - 1);

        counter.advance(3);

        assert_eq!(counter.now(), 1);
    }
}
