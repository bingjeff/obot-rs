use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

const INITIAL_STACK: usize = 0x2000_0000 + 96 * 1024;
const SCB_CPACR: usize = 0xE000_ED88;
const FPU_CP10_CP11_FULL_ACCESS: u32 = (0b11 << 20) | (0b11 << 22);

type Handler = extern "C" fn();
type ResetHandler = extern "C" fn() -> !;

#[repr(C)]
struct VectorTable {
    initial_stack: usize,
    reset: ResetHandler,
    exceptions: [Handler; 14],
    irqs_before_usb_lp: [Handler; 20],
    usb_lp: Handler,
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
    exceptions: [
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        default_handler,
        systick_handler,
    ],
    irqs_before_usb_lp: [default_handler; 20],
    usb_lp: usb_lp_handler,
};

#[unsafe(no_mangle)]
pub extern "C" fn Reset() -> ! {
    enable_fpu();
    init_memory();
    obot_g474::memory::init_stack_watermark();
    super::firmware_main();
}

extern "C" fn default_handler() {
    loop {
        core::hint::spin_loop();
    }
}

extern "C" fn usb_lp_handler() {
    obot_g474::usb::interrupt();
}

extern "C" fn systick_handler() {
    super::fast_loop_interrupt();
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
