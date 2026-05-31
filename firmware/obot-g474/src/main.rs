#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", allow(dead_code))]

#[cfg(target_os = "none")]
mod clock;
#[cfg(target_os = "none")]
mod debug_report;
#[cfg(target_os = "none")]
mod startup;

#[cfg(target_os = "none")]
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "none")]
use obot_core::benchmark::{BenchmarkReport, LoopBenchmark};
use obot_core::{
    Controller, Limits,
    timing::{LoopScheduler, LoopTiming},
};
#[cfg(target_os = "none")]
use obot_core::{
    current::CurrentCalibration,
    foc::{FocCommand, FocController, FocDesired, FocMeasured, FocParam},
    hall::HallElectricalAngle,
    output::OutputSafetyStatus,
    power::OutputGate,
};
#[cfg(target_os = "none")]
use obot_g474::adc::CurrentAdc;
#[cfg(target_os = "none")]
use obot_g474::cycle_counter::{CycleCounter, DwtCycleCounter};
#[cfg(target_os = "none")]
use obot_g474::driver::MotorDriverPins;
#[cfg(target_os = "none")]
use obot_g474::hall::HallInputs;
#[cfg(target_os = "none")]
use obot_g474::pwm::SafeZeroPwm;
#[cfg(target_os = "none")]
use obot_protocol::BenchmarkPacket;

#[cfg(target_os = "none")]
const FAST_LOOP_DT_S: f32 = 1.0 / 50_000.0;

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
    let mut benchmark_sequence = 0;
    if clock::configure_170mhz_hsi().is_err() {
        loop {
            core::hint::spin_loop();
        }
    }

    let cycle_counter = DwtCycleCounter::new();
    cycle_counter.enable();
    let driver = MotorDriverPins::init_motor_hall_disabled();
    let pwm = SafeZeroPwm::init_motor_hall();
    let mut hall = HallInputs::init_motor_hall();
    let current_adc = match CurrentAdc::init_motor_hall() {
        Ok(adc) => adc,
        Err(_) => loop {
            core::hint::spin_loop();
        },
    };
    let current_calibration = CurrentCalibration::MOTOR_HALL;
    let output_gate = OutputGate::MOTOR_HALL;
    let mut bus_voltage_raw = 0_u16;
    let mut output_allowed = false;
    let hall_angle = HallElectricalAngle::MOTOR_HALL;
    let mut foc = FocController::new(FocParam::MOTOR_HALL, FAST_LOOP_DT_S);
    foc.current_mode();

    let _ = controller.state();
    core::hint::black_box(pwm.config());

    loop {
        let poll = scheduler.poll(cycle_counter.now());
        if poll.fast {
            run_measured_loop(&mut fast_benchmark, &cycle_counter, || {
                pwm.write_zero_voltage();
                let hall_sample = hall.read_sample();
                let hall_sincos = hall_angle.sincos_hall_count(hall_sample.hall_count);
                let currents = current_calibration.convert(current_adc.read_samples());
                let foc_command = FocCommand {
                    desired: FocDesired::default(),
                    measured: FocMeasured {
                        currents,
                        motor_electrical_angle: hall_angle.electrical_radians(hall_sample.count),
                    },
                };
                let foc_status =
                    foc.step_with_sincos(&foc_command, hall_sincos.sin, hall_sincos.cos);
                let pwm_compares = pwm.compares_from_voltages(foc_status.command);
                core::hint::black_box(foc_status);
                core::hint::black_box(pwm_compares);
                core::hint::black_box(output_allowed);
                core::hint::black_box(controller.state());
            });
        }

        if poll.main {
            run_measured_loop(&mut main_benchmark, &cycle_counter, || {
                bus_voltage_raw = monitor_bus_voltage(&current_adc, output_gate);
                output_allowed = update_output_safety(
                    &driver,
                    output_gate.allows_output_raw(bus_voltage_raw),
                    controller.state().fault.is_some(),
                );
                core::hint::black_box(controller.state());
            });
            benchmark_sequence = publish_benchmark_report(
                benchmark_sequence,
                BenchmarkReport::from_loops(fast_benchmark, main_benchmark),
            );
        }

        if !poll.fast && !poll.main {
            core::hint::spin_loop();
        }
    }
}

#[cfg(target_os = "none")]
#[inline(never)]
fn update_output_safety(
    driver: &MotorDriverPins,
    bus_allows_output: bool,
    controller_faulted: bool,
) -> bool {
    static DRIVER_FAULT_LATCHED: AtomicBool = AtomicBool::new(false);

    let driver_status = driver.status();
    if driver_status.faulted {
        DRIVER_FAULT_LATCHED.store(true, Ordering::Relaxed);
    }

    let driver_fault_latched = DRIVER_FAULT_LATCHED.load(Ordering::Relaxed);
    let status = OutputSafetyStatus {
        output_allowed: false,
        command_blocked: true,
        bus_blocked: !bus_allows_output,
        driver_not_enabled: !driver_status.enabled,
        driver_fault_latched,
        controller_faulted,
    };
    core::hint::black_box(status);
    status.output_allowed
}

#[cfg(target_os = "none")]
#[inline(never)]
fn monitor_bus_voltage(current_adc: &CurrentAdc, output_gate: OutputGate) -> u16 {
    let bus_voltage_raw = current_adc.read_bus_voltage_raw();
    let output_allowed = output_gate.allows_output_raw(bus_voltage_raw);
    core::hint::black_box((bus_voltage_raw, output_allowed));
    bus_voltage_raw
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
fn publish_benchmark_report(sequence: u8, report: BenchmarkReport) -> u8 {
    let packet = BenchmarkPacket { sequence, report };
    debug_report::publish(packet);
    core::hint::black_box((debug_report::packet_ptr(), debug_report::packet_len()));
    sequence.wrapping_add(1)
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
