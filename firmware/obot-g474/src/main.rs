#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", allow(dead_code))]

#[cfg(target_os = "none")]
mod startup;

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use obot_core::benchmark::LoopBenchmark;
use obot_core::{
    Controller, Limits,
    timing::{LoopScheduler, LoopTiming},
};
#[cfg(target_os = "none")]
use obot_g474::cycle_counter::{CycleCounter, DwtCycleCounter};

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
fn firmware_main() -> ! {
    let controller = controller();
    let mut scheduler = scheduler();
    let mut fast_benchmark = LoopBenchmark::new();
    let mut main_benchmark = LoopBenchmark::new();
    let cycle_counter = DwtCycleCounter::new();
    cycle_counter.enable();

    let _ = controller.state();

    loop {
        let poll = scheduler.poll(cycle_counter.now());
        if poll.fast {
            run_measured_loop(&mut fast_benchmark, &cycle_counter, || {
                core::hint::black_box(controller.state());
            });
        }

        if poll.main {
            run_measured_loop(&mut main_benchmark, &cycle_counter, || {
                core::hint::black_box(controller.state());
            });
        }

        if !poll.fast && !poll.main {
            core::hint::spin_loop();
        }
    }
}

#[cfg(target_os = "none")]
fn run_measured_loop(
    benchmark: &mut LoopBenchmark,
    cycle_counter: &impl CycleCounter,
    work: impl FnOnce(),
) {
    let sample = benchmark.start(cycle_counter.now());
    work();
    benchmark.finish(sample, cycle_counter.now());
    core::hint::black_box(benchmark.execution());
    core::hint::black_box(benchmark.period());
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
