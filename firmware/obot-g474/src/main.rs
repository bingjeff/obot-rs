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
    cell::UnsafeCell,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "none")]
use obot_core::benchmark::{BenchmarkReport, LoopBenchmark};
use obot_core::{
    ControlMode, Controller, Limits,
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
use obot_g474::drv8323s::{Drv8323s, Drv8323sConfigReport};
#[cfg(target_os = "none")]
use obot_g474::hall::HallInputs;
#[cfg(target_os = "none")]
use obot_g474::pwm::SafeZeroPwm;
#[cfg(target_os = "none")]
use obot_protocol::{BenchmarkPacket, DriverCommand, DriverReportPacket, StatusPacket};

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
    let mut scheduler = scheduler();
    let mut fast_benchmark = LoopBenchmark::new();
    let mut main_benchmark = LoopBenchmark::new();
    let mut benchmark_sequence = 0;
    let mut status_sequence = 0;
    let mut command_sequence = 0;
    let mut driver_command_sequence = 0;
    let mut driver_report_sequence = 0;
    if clock::configure_170mhz_hsi().is_err() {
        loop {
            core::hint::spin_loop();
        }
    }

    let cycle_counter = DwtCycleCounter::new();
    cycle_counter.enable();
    let driver = MotorDriverPins::init_motor_hall_disabled();
    let driver_spi = Drv8323s::init_motor_hall();
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
    let mut command_allows_output = false;
    let mut controller_faulted = false;
    let hall_angle = HallElectricalAngle::MOTOR_HALL;
    let mut foc = FocController::new(FocParam::MOTOR_HALL, FAST_LOOP_DT_S);
    foc.current_mode();

    core::hint::black_box(pwm.config());

    loop {
        let poll = scheduler.poll(cycle_counter.now());
        if poll.fast {
            run_measured_loop(&mut fast_benchmark, &cycle_counter, || {
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
                let pwm_compares =
                    pwm.write_gated_voltage_commands_disabled(foc_status.command, output_allowed);
                core::hint::black_box(foc_status);
                core::hint::black_box(pwm_compares);
                core::hint::black_box(output_allowed);
            });
        }

        if poll.main {
            let mut driver_action_completed = false;
            run_measured_loop(&mut main_benchmark, &cycle_counter, || {
                (command_allows_output, controller_faulted) =
                    service_host_debug(&mut command_sequence, status_sequence);
                status_sequence = status_sequence.wrapping_add(1);
                driver_action_completed = service_driver_debug(
                    &driver,
                    &driver_spi,
                    &cycle_counter,
                    &mut driver_command_sequence,
                    &mut driver_report_sequence,
                );
                bus_voltage_raw = monitor_bus_voltage(&current_adc, output_gate);
                output_allowed = update_output_safety(
                    &driver,
                    command_allows_output,
                    output_gate.allows_output_raw(bus_voltage_raw),
                    controller_faulted,
                );
                core::hint::black_box((command_allows_output, controller_faulted));
            });
            if driver_action_completed {
                fast_benchmark = LoopBenchmark::new();
                main_benchmark = LoopBenchmark::new();
                benchmark_sequence = 0;
            } else {
                benchmark_sequence = publish_benchmark_report(
                    benchmark_sequence,
                    BenchmarkReport::from_loops(fast_benchmark, main_benchmark),
                );
            }
        }

        if !poll.fast && !poll.main {
            core::hint::spin_loop();
        }
    }
}

#[cfg(target_os = "none")]
struct ControllerStorage(UnsafeCell<Controller>);

#[cfg(target_os = "none")]
unsafe impl Sync for ControllerStorage {}

#[cfg(target_os = "none")]
static CONTROLLER: ControllerStorage = ControllerStorage(UnsafeCell::new(Controller::new(LIMITS)));

#[cfg(target_os = "none")]
static COMMAND_ALLOWS_OUTPUT: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "none")]
#[inline(never)]
fn service_host_debug(command_sequence: &mut u8, status_sequence: u8) -> (bool, bool) {
    let controller = controller_storage_mut();
    if let Some(packet) = debug_report::poll_command(command_sequence) {
        let command_allows_output = apply_host_command(controller, packet.command);
        COMMAND_ALLOWS_OUTPUT.store(command_allows_output, Ordering::Relaxed);
    }

    let state = controller.state();
    publish_status_report(status_sequence, state);
    (
        COMMAND_ALLOWS_OUTPUT.load(Ordering::Relaxed),
        state.fault.is_some(),
    )
}

#[cfg(target_os = "none")]
#[cold]
#[inline(never)]
fn service_driver_debug(
    driver: &MotorDriverPins,
    driver_spi: &Drv8323s,
    cycle_counter: &impl CycleCounter,
    command_sequence: &mut u8,
    report_sequence: &mut u8,
) -> bool {
    let Some(packet) = debug_report::poll_driver_command(command_sequence) else {
        return false;
    };

    let report = match packet.command {
        DriverCommand::Disable => {
            driver.disable();
            Drv8323sConfigReport::default()
        }
        DriverCommand::ConfigureEnable => {
            driver.enable();
            wait_cycles(cycle_counter, 1_700_000);
            let report = driver_spi.configure_motor_hall_registers();
            if !report.configured() {
                driver.disable();
            }
            report
        }
    };

    publish_driver_report(*report_sequence, report);
    *report_sequence = (*report_sequence).wrapping_add(1);
    core::hint::black_box((packet.command, report.configured()));
    true
}

#[cfg(target_os = "none")]
#[cold]
#[inline(never)]
fn wait_cycles(cycle_counter: &impl CycleCounter, cycles: u32) {
    let start = cycle_counter.now();
    while cycle_counter.now().wrapping_sub(start) < cycles {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
fn publish_driver_report(sequence: u8, report: Drv8323sConfigReport) {
    debug_report::publish_driver_report(DriverReportPacket {
        sequence,
        configured: report.configured(),
        verify_error_mask: report.verify_error_mask,
        transfer_error_mask: report.transfer_error_mask,
        status_before: report.status_before.map_or(0, |status| status.as_u32()),
        status_after: report.status_after.map_or(0, |status| status.as_u32()),
    });
    core::hint::black_box((
        debug_report::driver_report_packet_ptr(),
        debug_report::driver_report_packet_len(),
    ));
}

fn controller_storage_mut() -> &'static mut Controller {
    // SAFETY: The current firmware is single-threaded at this layer: command
    // polling/status publication happen from the main-loop branch only, and no
    // interrupt handler accesses this controller storage. Keeping it out of
    // `firmware_main` avoids perturbing the measured 50 kHz fast-loop frame.
    unsafe { &mut *CONTROLLER.0.get() }
}

#[cfg(target_os = "none")]
fn apply_host_command(controller: &mut Controller, command: obot_core::MotorCommand) -> bool {
    let mode = command.mode;
    let command_accepted = controller.apply(command).is_ok();
    let command_allows_output = command_accepted
        && matches!(
            mode,
            ControlMode::Torque | ControlMode::Velocity | ControlMode::Position
        );
    core::hint::black_box((command_accepted, command_allows_output));
    command_allows_output
}

#[cfg(target_os = "none")]
fn publish_status_report(sequence: u8, state: obot_core::MotorState) {
    debug_report::publish_status(StatusPacket { sequence, state });
    core::hint::black_box((
        debug_report::status_packet_ptr(),
        debug_report::status_packet_len(),
    ));
}

#[cfg(target_os = "none")]
#[inline(never)]
fn update_output_safety(
    driver: &MotorDriverPins,
    command_allows_output: bool,
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
        output_allowed: command_allows_output
            && bus_allows_output
            && driver_status.enabled
            && !driver_fault_latched
            && !controller_faulted,
        command_blocked: !command_allows_output,
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
