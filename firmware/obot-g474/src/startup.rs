use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

const INITIAL_STACK: usize = 0x2000_0000 + 96 * 1024;
const SCB_CPACR: usize = 0xE000_ED88;
const FPU_CP10_CP11_FULL_ACCESS: u32 = (0b11 << 20) | (0b11 << 22);

#[repr(C)]
struct VectorTable {
    initial_stack: usize,
    reset: extern "C" fn() -> !,
}

unsafe extern "C" {
    static _sidata: u32;
    static mut _sdata: u32;
    static mut _edata: u32;
    static mut _sbss: u32;
    static mut _ebss: u32;
}

#[used]
#[unsafe(link_section = ".vector_table.reset")]
static VECTOR_TABLE: VectorTable = VectorTable {
    initial_stack: INITIAL_STACK,
    reset: Reset,
};

#[unsafe(no_mangle)]
pub extern "C" fn Reset() -> ! {
    enable_fpu();
    init_memory();
    super::firmware_main();
}

fn enable_fpu() {
    let cpacr = SCB_CPACR as *mut u32;
    // SAFETY: CPACR is the ARMv7-M System Control Block register that enables
    // coprocessor access. This runs at reset before any floating-point code.
    unsafe {
        let value = read_volatile(cpacr);
        write_volatile(cpacr, value | FPU_CP10_CP11_FULL_ACCESS);
        core::arch::asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
    }
}

fn init_memory() {
    let mut src = addr_of!(_sidata);
    let mut dest = addr_of_mut!(_sdata);
    let data_end = addr_of_mut!(_edata);

    while dest < data_end {
        // SAFETY: The linker script defines a valid initialized-data load range
        // in flash and destination range in RAM. This runs before Rust code that
        // may read initialized statics.
        let value = unsafe { read_volatile(src) };
        // SAFETY: `dest` walks the linker-defined `.data` RAM range exactly once.
        unsafe { write_volatile(dest, value) };
        src = src.wrapping_add(1);
        dest = dest.wrapping_add(1);
    }

    let mut bss = addr_of_mut!(_sbss);
    let bss_end = addr_of_mut!(_ebss);
    while bss < bss_end {
        // SAFETY: `bss` walks the linker-defined `.bss` RAM range exactly once.
        unsafe { write_volatile(bss, 0) };
        bss = bss.wrapping_add(1);
    }
}
