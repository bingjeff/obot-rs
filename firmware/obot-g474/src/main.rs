#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", allow(dead_code))]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
use obot_core::{
    Controller, Limits,
    timing::{LoopScheduler, LoopTiming},
};

const LIMITS: Limits = Limits {
    max_torque_nm: 2.0,
    max_velocity_rad_s: 50.0,
    min_position_rad: -3.15,
    max_position_rad: 3.15,
};

fn controller() -> Controller {
    Controller::new(LIMITS)
}

fn scheduler() -> LoopScheduler {
    LoopScheduler::start(0, LoopTiming::OBOT_G474)
}

#[cfg(not(target_os = "none"))]
fn main() {
    let controller = controller();
    let mut scheduler = scheduler();
    let _ = controller.state();
    let _ = scheduler.poll(0);
}

#[cfg(target_os = "none")]
fn main() -> ! {
    let controller = controller();
    let mut scheduler = scheduler();
    let _ = controller.state();
    let _ = scheduler.poll(0);

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
