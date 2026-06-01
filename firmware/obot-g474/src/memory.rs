#[cfg(target_os = "none")]
use core::{
    ptr::{addr_of, addr_of_mut, read_volatile, write_volatile},
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(target_os = "none")]
const STACK_PATTERN: u32 = 0xA5A5_A5A5;
#[cfg(target_os = "none")]
const STACK_PAINT_GUARD_BYTES: usize = 512;

#[cfg(target_os = "none")]
static STACK_SCAN_START: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_os = "none")]
unsafe extern "C" {
    static mut _ebss: u32;
    static _estack: u32;
}

#[cfg(target_os = "none")]
pub fn init_stack_watermark() {
    let (start, _) = stack_bounds();
    let stack_pointer = current_stack_pointer();
    let paint_end = stack_pointer.saturating_sub(STACK_PAINT_GUARD_BYTES) & !0b11;
    if paint_end <= start {
        STACK_SCAN_START.store(start, Ordering::Relaxed);
        return;
    }

    let mut address = start;
    while address < paint_end {
        // SAFETY: `address` walks the inactive RAM region between .bss and the
        // current stack pointer, leaving a guard below the active stack frame.
        unsafe { write_volatile(address as *mut u32, STACK_PATTERN) };
        address += core::mem::size_of::<u32>();
    }
    STACK_SCAN_START.store(start, Ordering::Relaxed);
}

#[cfg(not(target_os = "none"))]
pub fn init_stack_watermark() {}

#[cfg(target_os = "none")]
pub fn stack_free_bytes() -> u32 {
    let (default_start, end) = stack_bounds();
    let start = STACK_SCAN_START.load(Ordering::Relaxed);
    let start = if start == 0 { default_start } else { start };
    let mut address = start;
    while address < end {
        // SAFETY: `address` walks the linker-defined RAM stack region. Volatile
        // reads are required because stack writes happen outside this function.
        let value = unsafe { read_volatile(address as *const u32) };
        if value != STACK_PATTERN {
            break;
        }
        address += core::mem::size_of::<u32>();
    }
    saturating_usize_to_u32(address.saturating_sub(start))
}

#[cfg(not(target_os = "none"))]
pub fn stack_free_bytes() -> u32 {
    0
}

#[cfg(target_os = "none")]
pub fn stack_used_bytes() -> u32 {
    let (start, end) = stack_bounds();
    saturating_usize_to_u32(end.saturating_sub(start)).saturating_sub(stack_free_bytes())
}

#[cfg(not(target_os = "none"))]
pub fn stack_used_bytes() -> u32 {
    0
}

pub const fn heap_free_bytes() -> u32 {
    0
}

pub const fn heap_used_bytes() -> u32 {
    0
}

#[cfg(target_os = "none")]
fn stack_bounds() -> (usize, usize) {
    let start = align_up(addr_of_mut!(_ebss) as usize, core::mem::align_of::<u32>());
    let end = addr_of!(_estack) as usize & !0b11;
    (start, end)
}

#[cfg(target_os = "none")]
fn current_stack_pointer() -> usize {
    let stack_pointer: usize;
    // SAFETY: This reads the current stack pointer into a general-purpose
    // register without modifying memory or flags.
    unsafe {
        core::arch::asm!("mov {}, sp", out(reg) stack_pointer, options(nomem, nostack, preserves_flags));
    }
    stack_pointer
}

#[cfg(target_os = "none")]
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(target_os = "none")]
fn saturating_usize_to_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_reports_zero_without_allocator() {
        assert_eq!(heap_free_bytes(), 0);
        assert_eq!(heap_used_bytes(), 0);
    }
}
