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
    sync::atomic::{AtomicU32, Ordering},
};
#[cfg(any(target_os = "none", test))]
use obot_core::ControlMode;
#[cfg(target_os = "none")]
use obot_core::MotorCommand;
#[cfg(target_os = "none")]
use obot_core::benchmark::{BenchmarkReport, LoopBenchmark};
use obot_core::{
    Controller, Limits,
    timing::{LoopScheduler, LoopTiming},
};
#[cfg(target_os = "none")]
use obot_core::{
    current::CurrentCalibration,
    foc::{FocDesired, MotorHallFocController},
    hall::{HallElectricalAngle, HallMotionEstimate, HallMotionEstimator},
    host::{HostCommandWatchdog, HostCommandWatchdogStatus},
    outer_loop::{MotorHallOuterLoop, MotorHallOuterLoopParam},
    output::{OutputSafety, OutputSafetyInputs, OutputSafetyStatus},
    power::OutputGate,
    text_api::{ApiDispatchError, ApiRequest, ApiValue, format_value, parse_request},
};
#[cfg(target_os = "none")]
use obot_g474::adc::CurrentAdc;
#[cfg(target_os = "none")]
use obot_g474::cycle_counter::CycleCounter;
#[cfg(target_os = "none")]
use obot_g474::driver::MotorDriverPins;
#[cfg(target_os = "none")]
use obot_g474::drv8323s::{Drv8323s, Drv8323sConfigReport};
#[cfg(target_os = "none")]
use obot_g474::hall::HallInputs;
#[cfg(target_os = "none")]
use obot_g474::pwm::{BridgeOutputStatus, SafeZeroPwm};
#[cfg(target_os = "none")]
use obot_protocol::{
    BenchmarkPacket, BusVoltagePacket, DriverCommand, DriverReportPacket, OutputSafetyPacket,
    StatusPacket, TEXT_API_PAYLOAD_LEN, TextApiResponsePacket, TextApiResponseStatus,
};

#[cfg(target_os = "none")]
const FAST_LOOP_DT_S: f32 = 1.0 / 50_000.0;
#[cfg(target_os = "none")]
const FAST_LOOP_PERIOD_CYCLES: u32 = 3_400;
#[cfg(target_os = "none")]
const MAIN_LOOP_FAST_TICKS: u32 = 5;
#[cfg(target_os = "none")]
const SYSTICK_CSR: *mut u32 = 0xE000_E010 as *mut u32;
#[cfg(target_os = "none")]
const SYSTICK_RVR: *mut u32 = 0xE000_E014 as *mut u32;
#[cfg(target_os = "none")]
const SYSTICK_CVR: *mut u32 = 0xE000_E018 as *mut u32;
#[cfg(target_os = "none")]
const SCB_ICSR: *mut u32 = 0xE000_ED04 as *mut u32;
#[cfg(target_os = "none")]
const SYSTICK_CSR_ENABLE: u32 = 1 << 0;
#[cfg(target_os = "none")]
const SYSTICK_CSR_TICKINT: u32 = 1 << 1;
#[cfg(target_os = "none")]
const SYSTICK_CSR_CLKSOURCE_PROCESSOR: u32 = 1 << 2;
#[cfg(target_os = "none")]
const SCB_ICSR_PENDSTSET: u32 = 1 << 26;
#[cfg(target_os = "none")]
const HOST_COMMAND_TIMEOUT_MAIN_TICKS: u32 = 1_000;
#[cfg(target_os = "none")]
const DRIVER_ENABLE_SETTLE_MAIN_TICKS: u16 = 100;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_BUS_BLOCKED_BIT: u32 = 1 << 0;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_DRIVER_NOT_ENABLED_BIT: u32 = 1 << 1;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_DRIVER_FAULT_BIT: u32 = 1 << 2;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_CONTROLLER_FAULT_BIT: u32 = 1 << 3;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_HOST_TIMED_OUT_BIT: u32 = 1 << 4;
#[cfg(target_os = "none")]
const BRIDGE_PREARM_OUTPUT_ALREADY_ENABLED_BIT: u32 = 1 << 5;

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
    let mut next_main_fast_tick = MAIN_LOOP_FAST_TICKS;
    let mut main_benchmark = LoopBenchmark::new();
    let mut benchmark_sequence = 0;
    let mut status_sequence = 0;
    let mut command_sequence = 0;
    let mut driver_command_sequence = 0;
    let mut driver_report_sequence = 0;
    let mut output_safety_sequence = 0;
    let mut bus_voltage_sequence = 0;
    let mut text_api_request_sequence = 0;
    let mut last_driver_report = Drv8323sConfigReport::default();
    let mut driver_debug_service = DriverDebugService::default();
    let mut last_benchmark_report = BenchmarkReport::default();
    let mut host_watchdog = HostCommandWatchdog::new(HOST_COMMAND_TIMEOUT_MAIN_TICKS);
    if clock::configure_170mhz_hsi().is_err() {
        loop {
            core::hint::spin_loop();
        }
    }
    if clock::configure_usb_hsi48_crs().is_err() {
        loop {
            core::hint::spin_loop();
        }
    }
    let usb = obot_g474::usb::UsbDevice::prepare_disconnected();

    let cycle_counter = SysTickCycleCounter;
    let main_cycle_counter = SysTickMainCycleCounter;
    let driver = MotorDriverPins::init_motor_hall_disabled();
    let driver_spi = Drv8323s::init_motor_hall();
    let pwm = SafeZeroPwm::init_motor_hall();
    let hall = HallInputs::init_motor_hall();
    let current_adc = match CurrentAdc::init_motor_hall() {
        Ok(adc) => adc,
        Err(_) => loop {
            core::hint::spin_loop();
        },
    };
    let bus_voltage_adc = current_adc;
    let output_gate = OutputGate::MOTOR_HALL;
    let mut bus_voltage_raw = 0_u16;
    let mut output_allowed = false;
    let mut hall_motion = HallMotionEstimator::new(42.0, -1.0, 10_000.0, 0.005);
    let mut outer_loop =
        MotorHallOuterLoop::new(MotorHallOuterLoopParam::MOTOR_HALL, 1.0 / 10_000.0);
    let mut foc_desired = outer_loop.desired_from_state(
        obot_core::MotorState::default(),
        HallMotionEstimate::default(),
    );
    let mut foc = MotorHallFocController::new(FAST_LOOP_DT_S);
    foc.initialize();
    usb.connect();

    core::hint::black_box(pwm.config());
    install_fast_loop_context(FastLoopContext::new(
        cycle_counter,
        pwm,
        hall,
        current_adc,
        foc,
        foc_desired,
        output_allowed,
    ));
    start_fast_loop_interrupt();

    loop {
        if main_loop_due(&mut next_main_fast_tick) {
            let mut driver_action_completed = false;
            run_preemptible_main_loop(&mut main_benchmark, &main_cycle_counter, || {
                let host_poll = service_host_debug(&mut command_sequence);
                let host_status =
                    update_host_watchdog(&mut host_watchdog, host_poll.command_allows_output);
                if host_status.timed_out {
                    force_controller_disabled();
                }
                let controller_state = controller_storage_mut().state();
                let hall_count = fast_loop_hall_count();
                let hall_feedback = hall_motion.update(obot_core::hall::HallSample {
                    count: hall_count,
                    hall_count: 0,
                });
                obot_g474::usb::publish_hall_feedback(hall_feedback);
                foc_desired = outer_loop.desired_from_state(controller_state, hall_feedback);
                publish_status_report(status_sequence, controller_state);
                status_sequence = status_sequence.wrapping_add(1);
                if let Some(report) = driver_debug_service.service(
                    &driver,
                    &driver_spi,
                    &mut driver_command_sequence,
                    &mut driver_report_sequence,
                ) {
                    last_driver_report = report;
                    driver_action_completed = true;
                }
                bus_voltage_raw = monitor_bus_voltage(&bus_voltage_adc, output_gate);
                bus_voltage_sequence =
                    publish_bus_voltage_report(bus_voltage_sequence, bus_voltage_raw);
                let output_safety_status = update_output_safety(
                    &driver,
                    host_status.output_allowed,
                    output_gate.allows_output_raw(bus_voltage_raw),
                    controller_state.fault.is_some(),
                    host_status.timed_out,
                    host_poll.clear_output_safety_faults,
                );
                output_allowed = output_safety_status.output_allowed;
                set_fast_loop_command(foc_desired, output_allowed);
                output_safety_sequence =
                    publish_output_safety_report(output_safety_sequence, output_safety_status);
                let bridge_output_status = fast_loop_bridge_output_status();
                obot_g474::usb::publish_hrtim_output_status(
                    bridge_output_status.disable_status,
                    bridge_output_status.all_disabled,
                    bridge_output_status.all_enabled,
                );
                let bridge_prearm_blockers =
                    bridge_prearm_blockers(output_safety_status, bridge_output_status);
                obot_g474::usb::publish_bridge_prearm_status(bridge_prearm_blockers);
                service_text_api_debug(
                    &mut text_api_request_sequence,
                    last_benchmark_report,
                    controller_state,
                    last_driver_report,
                    output_safety_status,
                    bus_voltage_raw,
                    hall_feedback,
                    bridge_output_status,
                    bridge_prearm_blockers,
                );
                core::hint::black_box((host_poll, host_status, controller_state.fault));
            });
            if driver_action_completed {
                reset_fast_loop_benchmark();
                main_benchmark = LoopBenchmark::new();
                benchmark_sequence = 0;
                last_benchmark_report = BenchmarkReport::default();
            } else {
                last_benchmark_report =
                    BenchmarkReport::from_loops(fast_loop_benchmark(), main_benchmark);
                obot_g474::usb::publish_text_api_benchmark(last_benchmark_report);
                benchmark_sequence =
                    publish_benchmark_report(benchmark_sequence, last_benchmark_report);
            }
        } else {
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
struct OutputSafetyStorage(UnsafeCell<OutputSafety>);

#[cfg(target_os = "none")]
unsafe impl Sync for OutputSafetyStorage {}

#[cfg(target_os = "none")]
static OUTPUT_SAFETY: OutputSafetyStorage =
    OutputSafetyStorage(UnsafeCell::new(OutputSafety::new()));

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SysTickCycleCounter;

#[cfg(target_os = "none")]
impl CycleCounter for SysTickCycleCounter {
    fn now(&self) -> u32 {
        systick_cycles_now(false)
    }
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SysTickMainCycleCounter;

#[cfg(target_os = "none")]
impl CycleCounter for SysTickMainCycleCounter {
    fn now(&self) -> u32 {
        systick_cycles_now(true)
    }
}

#[cfg(target_os = "none")]
fn systick_cycles_now(account_pending_tick: bool) -> u32 {
    loop {
        let ticks_before = FAST_LOOP_TICKS.load(Ordering::Relaxed);
        let mut current = unsafe { core::ptr::read_volatile(SYSTICK_CVR) };
        let pending_tick = account_pending_tick
            && (unsafe { core::ptr::read_volatile(SCB_ICSR) } & SCB_ICSR_PENDSTSET) != 0;
        let ticks_after = FAST_LOOP_TICKS.load(Ordering::Relaxed);
        if ticks_before == ticks_after {
            let ticks = if pending_tick {
                current = unsafe { core::ptr::read_volatile(SYSTICK_CVR) };
                ticks_after.wrapping_add(1)
            } else {
                ticks_after
            };
            let elapsed_in_period = (FAST_LOOP_PERIOD_CYCLES - 1)
                .saturating_sub(current.min(FAST_LOOP_PERIOD_CYCLES - 1));
            return ticks
                .wrapping_mul(FAST_LOOP_PERIOD_CYCLES)
                .wrapping_add(elapsed_in_period);
        }
    }
}

#[cfg(target_os = "none")]
struct FastLoopStorage(UnsafeCell<Option<FastLoopContext>>);

#[cfg(target_os = "none")]
unsafe impl Sync for FastLoopStorage {}

#[cfg(target_os = "none")]
static FAST_LOOP: FastLoopStorage = FastLoopStorage(UnsafeCell::new(None));

#[cfg(target_os = "none")]
static FAST_LOOP_TICKS: AtomicU32 = AtomicU32::new(0);

#[cfg(target_os = "none")]
struct FastLoopContext {
    benchmark: LoopBenchmark,
    cycle_counter: SysTickCycleCounter,
    pwm: SafeZeroPwm,
    hall: HallInputs,
    current_adc: CurrentAdc,
    hall_count: i32,
    foc: MotorHallFocController,
    foc_desired: FocDesired,
    output_allowed: bool,
}

#[cfg(target_os = "none")]
impl FastLoopContext {
    fn new(
        cycle_counter: SysTickCycleCounter,
        pwm: SafeZeroPwm,
        hall: HallInputs,
        current_adc: CurrentAdc,
        foc: MotorHallFocController,
        foc_desired: FocDesired,
        output_allowed: bool,
    ) -> Self {
        Self {
            benchmark: LoopBenchmark::new(),
            cycle_counter,
            pwm,
            hall,
            current_adc,
            hall_count: 0,
            foc,
            foc_desired,
            output_allowed,
        }
    }

    fn run(&mut self) {
        let sample = self.benchmark.start(self.cycle_counter.now());
        let hall_sample = self.hall.read_sample();
        self.hall_count = hall_sample.count;
        let hall_sincos = HallElectricalAngle::motor_hall_sincos_hall_count(hall_sample.hall_count);
        let currents = CurrentCalibration::motor_hall_convert(self.current_adc.read_samples());
        let voltage_command = self.foc.step_voltage_command_with_sincos(
            self.foc_desired,
            currents,
            hall_sincos.sin,
            hall_sincos.cos,
        );
        let _ = self
            .pwm
            .write_gated_voltage_commands_disabled(voltage_command, self.output_allowed);
        self.benchmark.finish(sample, self.cycle_counter.now());
    }

    fn set_command(&mut self, foc_desired: FocDesired, output_allowed: bool) {
        self.foc_desired = foc_desired;
        self.output_allowed = output_allowed;
    }
}

#[cfg(target_os = "none")]
fn install_fast_loop_context(context: FastLoopContext) {
    interrupt_free(|| unsafe {
        *FAST_LOOP.0.get() = Some(context);
    });
}

#[cfg(target_os = "none")]
fn start_fast_loop_interrupt() {
    unsafe {
        core::ptr::write_volatile(SYSTICK_RVR, FAST_LOOP_PERIOD_CYCLES - 1);
        core::ptr::write_volatile(SYSTICK_CVR, 0);
        core::arch::asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
        core::ptr::write_volatile(
            SYSTICK_CSR,
            SYSTICK_CSR_ENABLE | SYSTICK_CSR_TICKINT | SYSTICK_CSR_CLKSOURCE_PROCESSOR,
        );
    }
}

#[cfg(target_os = "none")]
fn fast_loop_interrupt() {
    FAST_LOOP_TICKS.fetch_add(1, Ordering::Relaxed);
    if let Some(context) = fast_loop_context_mut() {
        context.run();
    }
}

#[cfg(target_os = "none")]
fn main_loop_due(next_main_fast_tick: &mut u32) -> bool {
    let ticks = FAST_LOOP_TICKS.load(Ordering::Relaxed);
    if (ticks.wrapping_sub(*next_main_fast_tick) as i32) < 0 {
        return false;
    }

    let elapsed = ticks.wrapping_sub(*next_main_fast_tick);
    let periods_elapsed = elapsed / MAIN_LOOP_FAST_TICKS + 1;
    *next_main_fast_tick =
        next_main_fast_tick.wrapping_add(MAIN_LOOP_FAST_TICKS.wrapping_mul(periods_elapsed));
    true
}

#[cfg(target_os = "none")]
fn set_fast_loop_command(foc_desired: FocDesired, output_allowed: bool) {
    interrupt_free(|| {
        if let Some(context) = fast_loop_context_mut() {
            context.set_command(foc_desired, output_allowed);
        }
    });
}

#[cfg(target_os = "none")]
fn reset_fast_loop_benchmark() {
    interrupt_free(|| {
        if let Some(context) = fast_loop_context_mut() {
            context.benchmark = LoopBenchmark::new();
        }
    });
}

#[cfg(target_os = "none")]
fn fast_loop_benchmark() -> LoopBenchmark {
    interrupt_free(|| {
        fast_loop_context_mut()
            .map(|context| context.benchmark)
            .unwrap_or_default()
    })
}

#[cfg(target_os = "none")]
fn fast_loop_hall_count() -> i32 {
    interrupt_free(|| {
        fast_loop_context_mut()
            .map(|context| context.hall_count)
            .unwrap_or_default()
    })
}

#[cfg(target_os = "none")]
fn fast_loop_bridge_output_status() -> BridgeOutputStatus {
    interrupt_free(|| {
        fast_loop_context_mut()
            .map(|context| context.pwm.bridge_output_status())
            .unwrap_or(BridgeOutputStatus {
                disable_status: 0,
                all_disabled: false,
                all_enabled: false,
            })
    })
}

#[cfg(target_os = "none")]
fn fast_loop_execution_total_cycles() -> u64 {
    fast_loop_context_mut()
        .map(|context| context.benchmark.execution().total_cycles())
        .unwrap_or_default()
}

#[cfg(target_os = "none")]
fn fast_loop_context_mut() -> Option<&'static mut FastLoopContext> {
    unsafe { (&mut *FAST_LOOP.0.get()).as_mut() }
}

#[cfg(target_os = "none")]
fn interrupt_free<R>(work: impl FnOnce() -> R) -> R {
    unsafe { core::arch::asm!("cpsid i", options(nomem, nostack, preserves_flags)) };
    let result = work();
    unsafe { core::arch::asm!("cpsie i", options(nomem, nostack, preserves_flags)) };
    result
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HostDebugPoll {
    command_allows_output: Option<bool>,
    clear_output_safety_faults: bool,
}

#[cfg(target_os = "none")]
#[inline(never)]
fn service_host_debug(command_sequence: &mut u8) -> HostDebugPoll {
    let Some(packet) = obot_g474::usb::poll_realtime_command()
        .or_else(|| debug_report::poll_command(command_sequence))
    else {
        return HostDebugPoll::default();
    };

    let (command_allows_output, clear_faults_accepted) =
        apply_host_command(controller_storage_mut(), packet.command);

    HostDebugPoll {
        command_allows_output: Some(command_allows_output),
        clear_output_safety_faults: clear_faults_accepted,
    }
}

#[cfg(target_os = "none")]
fn update_host_watchdog(
    watchdog: &mut HostCommandWatchdog,
    command_allows_output: Option<bool>,
) -> HostCommandWatchdogStatus {
    match command_allows_output {
        Some(command_allows_output) => watchdog.observe_command(command_allows_output),
        None => watchdog.tick(),
    }
}

#[cfg(target_os = "none")]
fn force_controller_disabled() {
    let _ = controller_storage_mut().apply(MotorCommand {
        mode: ControlMode::Disabled,
        ..MotorCommand::default()
    });
}

#[cfg(target_os = "none")]
#[inline(never)]
fn service_text_api_debug(
    request_sequence: &mut u8,
    benchmark_report: BenchmarkReport,
    controller_state: obot_core::MotorState,
    driver_report: Drv8323sConfigReport,
    output_safety_status: OutputSafetyStatus,
    bus_voltage_raw: u16,
    hall_feedback: HallMotionEstimate,
    bridge_output_status: BridgeOutputStatus,
    bridge_prearm_blockers: u32,
) {
    let Some(packet) = debug_report::poll_text_api_request(request_sequence) else {
        return;
    };

    let (status, response_len, response) = match core::str::from_utf8(packet.payload()) {
        Ok(request) => {
            let mut response = [0; TEXT_API_PAYLOAD_LEN];
            match dispatch_firmware_text_api(
                request,
                &mut response,
                benchmark_report,
                controller_state,
                driver_report,
                output_safety_status,
                bus_voltage_raw,
                hall_feedback,
                bridge_output_status,
                bridge_prearm_blockers,
            ) {
                Ok(response_text) => (TextApiResponseStatus::Ok, response_text.len(), response),
                Err(error) => (text_api_response_status(error), 0, response),
            }
        }
        Err(_) => (
            TextApiResponseStatus::InvalidUtf8,
            0,
            [0; TEXT_API_PAYLOAD_LEN],
        ),
    };

    let packet = TextApiResponsePacket::new(packet.sequence, status, &response[..response_len])
        .unwrap_or_else(|_| {
            TextApiResponsePacket::new(packet.sequence, TextApiResponseStatus::ResponseTooLong, &[])
                .unwrap()
        });
    debug_report::publish_text_api_response(packet);
    core::hint::black_box((
        debug_report::text_api_request_packet_ptr(),
        debug_report::text_api_request_packet_len(),
        debug_report::text_api_response_packet_ptr(),
        debug_report::text_api_response_packet_len(),
    ));
}

#[cfg(target_os = "none")]
const CPU_FREQUENCY_HZ: u32 = 170_000_000;

#[cfg(target_os = "none")]
const MESSAGES_VERSION: &str = "3.3";

#[cfg(target_os = "none")]
const TEXT_API_NAMES: &[&str] = &[
    "api_length",
    "cpu_frequency",
    "messages_version",
    "t_exec_fastloop",
    "t_exec_mainloop",
    "t_period_fastloop",
    "t_period_mainloop",
    "max_fast_loop_cycles",
    "max_fast_loop_period",
    "fast_max_load_percent",
    "fast_max_remaining_cycles",
    "max_main_loop_cycles",
    "max_main_loop_period",
    "main_max_load_percent",
    "main_max_remaining_cycles",
    "combined_max_cycles",
    "combined_max_load_percent",
    "combined_max_remaining_cycles",
    "mean_fast_loop_cycles",
    "mean_fast_loop_period",
    "mean_main_loop_cycles",
    "mean_main_loop_period",
    "combined_mean_cycles",
    "combined_mean_load_percent",
    "combined_mean_remaining_cycles",
    "fault",
    "torque_nm",
    "velocity_rad_s",
    "position_rad",
    "motor_position_raw",
    "motor_position_rad",
    "motor_velocity_rad_s",
    "motor_velocity_filtered_rad_s",
    "output_allowed",
    "command_blocked",
    "bus_blocked",
    "driver_not_enabled",
    "driver_fault_latched",
    "controller_faulted",
    "host_timed_out",
    "bus_voltage_raw",
    "bus_voltage_volts",
    "bus_allows_output",
    "bridge_output_disable_status",
    "bridge_outputs_disabled",
    "bridge_outputs_enabled",
    "bridge_prearm_ready",
    "bridge_prearm_blockers",
    "driver_configured",
    "verify_error_mask",
    "transfer_error_mask",
    "status_before",
    "status_after",
];

#[cfg(target_os = "none")]
fn dispatch_firmware_text_api<'out>(
    request: &str,
    output: &'out mut [u8],
    benchmark_report: BenchmarkReport,
    controller_state: obot_core::MotorState,
    driver_report: Drv8323sConfigReport,
    output_safety_status: OutputSafetyStatus,
    bus_voltage_raw: u16,
    hall_feedback: HallMotionEstimate,
    bridge_output_status: BridgeOutputStatus,
    bridge_prearm_blockers: u32,
) -> Result<&'out str, ApiDispatchError> {
    match parse_request(request).map_err(ApiDispatchError::Parse)? {
        ApiRequest::Get { name } => format_firmware_text_api_value(
            name,
            output,
            benchmark_report,
            controller_state,
            driver_report,
            output_safety_status,
            bus_voltage_raw,
            hall_feedback,
            bridge_output_status,
            bridge_prearm_blockers,
        ),
        ApiRequest::Set { name, .. } => {
            if firmware_text_api_name_index(name).is_some() {
                Err(ApiDispatchError::ReadOnly)
            } else {
                Err(ApiDispatchError::UnknownName)
            }
        }
        ApiRequest::NameAt { index } => {
            let name = TEXT_API_NAMES
                .get(index as usize)
                .ok_or(ApiDispatchError::NameIndexOutOfRange)?;
            format_value(ApiValue::Str(name), output)
        }
    }
}

#[cfg(target_os = "none")]
fn firmware_text_api_name_index(name: &str) -> Option<usize> {
    TEXT_API_NAMES
        .iter()
        .position(|candidate| *candidate == name)
}

#[cfg(target_os = "none")]
fn format_firmware_text_api_value<'out>(
    name: &str,
    output: &'out mut [u8],
    benchmark_report: BenchmarkReport,
    controller_state: obot_core::MotorState,
    driver_report: Drv8323sConfigReport,
    output_safety_status: OutputSafetyStatus,
    bus_voltage_raw: u16,
    hall_feedback: HallMotionEstimate,
    bridge_output_status: BridgeOutputStatus,
    bridge_prearm_blockers: u32,
) -> Result<&'out str, ApiDispatchError> {
    let combined_max_cycles = 5 * benchmark_report.max_fast_loop_cycles() as u64
        + benchmark_report.max_main_loop_cycles() as u64;
    let combined_mean_milli_cycles = 5 * benchmark_report.mean_fast_loop_cycles_milli_cycles()
        + benchmark_report.mean_main_loop_cycles_milli_cycles();
    let combined_mean_remaining_milli_cycles =
        17_000_i64 * 1_000 - combined_mean_milli_cycles as i64;

    let value = match name {
        "api_length" => ApiValue::U16(TEXT_API_NAMES.len() as u16),
        "cpu_frequency" => ApiValue::U32(CPU_FREQUENCY_HZ),
        "messages_version" => ApiValue::Str(MESSAGES_VERSION),
        "t_exec_fastloop" => ApiValue::U32(benchmark_report.t_exec_fastloop()),
        "t_exec_mainloop" => ApiValue::U32(benchmark_report.t_exec_mainloop()),
        "t_period_fastloop" => ApiValue::U32(benchmark_report.t_period_fastloop()),
        "t_period_mainloop" => ApiValue::U32(benchmark_report.t_period_mainloop()),
        "max_fast_loop_cycles" => ApiValue::U32(benchmark_report.max_fast_loop_cycles()),
        "max_fast_loop_period" => ApiValue::U32(benchmark_report.max_fast_loop_period_cycles()),
        "fast_max_load_percent" => ApiValue::Fixed3(percent_milli(
            benchmark_report.max_fast_loop_cycles() as u64,
            benchmark_report.max_fast_loop_period_cycles() as u64,
        )),
        "fast_max_remaining_cycles" => ApiValue::I32(
            benchmark_report.max_fast_loop_period_cycles() as i32
                - benchmark_report.max_fast_loop_cycles() as i32,
        ),
        "max_main_loop_cycles" => ApiValue::U32(benchmark_report.max_main_loop_cycles()),
        "max_main_loop_period" => ApiValue::U32(benchmark_report.max_main_loop_period_cycles()),
        "main_max_load_percent" => ApiValue::Fixed3(percent_milli(
            benchmark_report.max_main_loop_cycles() as u64,
            benchmark_report.max_main_loop_period_cycles() as u64,
        )),
        "main_max_remaining_cycles" => ApiValue::I32(
            benchmark_report.max_main_loop_period_cycles() as i32
                - benchmark_report.max_main_loop_cycles() as i32,
        ),
        "combined_max_cycles" => ApiValue::U32(combined_max_cycles as u32),
        "combined_max_load_percent" => ApiValue::Fixed3(percent_milli(combined_max_cycles, 17_000)),
        "combined_max_remaining_cycles" => ApiValue::I32(17_000 - combined_max_cycles as i32),
        "mean_fast_loop_cycles" => {
            ApiValue::Fixed3(benchmark_report.mean_fast_loop_cycles_milli_cycles() as i64)
        }
        "mean_fast_loop_period" => {
            ApiValue::Fixed3(benchmark_report.mean_fast_loop_period_milli_cycles() as i64)
        }
        "mean_main_loop_cycles" => {
            ApiValue::Fixed3(benchmark_report.mean_main_loop_cycles_milli_cycles() as i64)
        }
        "mean_main_loop_period" => {
            ApiValue::Fixed3(benchmark_report.mean_main_loop_period_milli_cycles() as i64)
        }
        "combined_mean_cycles" => ApiValue::Fixed3(combined_mean_milli_cycles as i64),
        "combined_mean_load_percent" => {
            ApiValue::Fixed3(milli_percent_milli(combined_mean_milli_cycles, 17_000))
        }
        "combined_mean_remaining_cycles" => ApiValue::Fixed3(combined_mean_remaining_milli_cycles),
        "fault" => ApiValue::Str(fault_name(controller_state.fault)),
        "torque_nm" => ApiValue::Fixed3((controller_state.torque_nm * 1_000.0) as i64),
        "velocity_rad_s" => ApiValue::Fixed3((controller_state.velocity_rad_s * 1_000.0) as i64),
        "position_rad" => ApiValue::Fixed3((controller_state.position_rad * 1_000.0) as i64),
        "motor_position_raw" => ApiValue::I32(hall_feedback.raw_count),
        "motor_position_rad" => ApiValue::Fixed3((hall_feedback.position_rad * 1_000.0) as i64),
        "motor_velocity_rad_s" => ApiValue::Fixed3((hall_feedback.velocity_rad_s * 1_000.0) as i64),
        "motor_velocity_filtered_rad_s" => {
            ApiValue::Fixed3((hall_feedback.velocity_filtered_rad_s * 1_000.0) as i64)
        }
        "output_allowed" => ApiValue::Bool(output_safety_status.output_allowed),
        "command_blocked" => ApiValue::Bool(output_safety_status.command_blocked),
        "bus_blocked" => ApiValue::Bool(output_safety_status.bus_blocked),
        "driver_not_enabled" => ApiValue::Bool(output_safety_status.driver_not_enabled),
        "driver_fault_latched" => ApiValue::Bool(output_safety_status.driver_fault_latched),
        "controller_faulted" => ApiValue::Bool(output_safety_status.controller_faulted),
        "host_timed_out" => ApiValue::Bool(output_safety_status.host_timed_out),
        "bus_voltage_raw" => ApiValue::U16(bus_voltage_raw),
        "bus_voltage_volts" => ApiValue::Fixed3(bus_voltage_millivolts(bus_voltage_raw)),
        "bus_allows_output" => {
            ApiValue::Bool(OutputGate::MOTOR_HALL.allows_output_raw(bus_voltage_raw))
        }
        "bridge_output_disable_status" => ApiValue::U32(bridge_output_status.disable_status),
        "bridge_outputs_disabled" => ApiValue::Bool(bridge_output_status.all_disabled),
        "bridge_outputs_enabled" => ApiValue::Bool(bridge_output_status.all_enabled),
        "bridge_prearm_ready" => ApiValue::Bool(bridge_prearm_blockers == 0),
        "bridge_prearm_blockers" => ApiValue::U32(bridge_prearm_blockers),
        "driver_configured" => ApiValue::Bool(driver_report.configured()),
        "verify_error_mask" => ApiValue::U16(driver_report.verify_error_mask),
        "transfer_error_mask" => ApiValue::U16(driver_report.transfer_error_mask),
        "status_before" => ApiValue::U32(
            driver_report
                .status_before
                .map_or(0, |status| status.as_u32()),
        ),
        "status_after" => ApiValue::U32(
            driver_report
                .status_after
                .map_or(0, |status| status.as_u32()),
        ),
        _ => return Err(ApiDispatchError::UnknownName),
    };

    format_value(value, output)
}

#[cfg(target_os = "none")]
fn text_api_response_status(error: ApiDispatchError) -> TextApiResponseStatus {
    match error {
        ApiDispatchError::Parse(_) => TextApiResponseStatus::ParseError,
        ApiDispatchError::UnknownName => TextApiResponseStatus::UnknownName,
        ApiDispatchError::ReadOnly => TextApiResponseStatus::ReadOnly,
        ApiDispatchError::NameIndexOutOfRange => TextApiResponseStatus::NameIndexOutOfRange,
        ApiDispatchError::ResponseTooLong => TextApiResponseStatus::ResponseTooLong,
    }
}

#[cfg(target_os = "none")]
fn fault_name(fault: Option<obot_core::Fault>) -> &'static str {
    match fault {
        None => "none",
        Some(obot_core::Fault::CommandNotFinite) => "command_not_finite",
        Some(obot_core::Fault::TorqueLimit) => "torque_limit",
        Some(obot_core::Fault::VelocityLimit) => "velocity_limit",
        Some(obot_core::Fault::PositionLimit) => "position_limit",
    }
}

#[cfg(target_os = "none")]
fn percent_milli(numerator_cycles: u64, denominator_cycles: u64) -> i64 {
    if denominator_cycles == 0 {
        return 0;
    }

    (numerator_cycles.saturating_mul(100_000) / denominator_cycles) as i64
}

#[cfg(target_os = "none")]
fn milli_percent_milli(numerator_milli_cycles: u64, denominator_cycles: u64) -> i64 {
    if denominator_cycles == 0 {
        return 0;
    }

    (numerator_milli_cycles.saturating_mul(100) / denominator_cycles) as i64
}

#[cfg(target_os = "none")]
fn bus_voltage_millivolts(raw: u16) -> i64 {
    raw as i64 * 8_000 / OutputGate::MOTOR_HALL.min_raw as i64
}

#[cfg(target_os = "none")]
fn bridge_prearm_blockers(
    output_safety_status: OutputSafetyStatus,
    bridge_output_status: BridgeOutputStatus,
) -> u32 {
    bool_mask(
        output_safety_status.bus_blocked,
        BRIDGE_PREARM_BUS_BLOCKED_BIT,
    ) | bool_mask(
        output_safety_status.driver_not_enabled,
        BRIDGE_PREARM_DRIVER_NOT_ENABLED_BIT,
    ) | bool_mask(
        output_safety_status.driver_fault_latched,
        BRIDGE_PREARM_DRIVER_FAULT_BIT,
    ) | bool_mask(
        output_safety_status.controller_faulted,
        BRIDGE_PREARM_CONTROLLER_FAULT_BIT,
    ) | bool_mask(
        output_safety_status.host_timed_out,
        BRIDGE_PREARM_HOST_TIMED_OUT_BIT,
    ) | bool_mask(
        bridge_output_status.all_enabled,
        BRIDGE_PREARM_OUTPUT_ALREADY_ENABLED_BIT,
    )
}

#[cfg(target_os = "none")]
fn bool_mask(value: bool, bit: u32) -> u32 {
    if value { bit } else { 0 }
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DriverDebugService {
    pending: Option<PendingDriverAction>,
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingDriverAction {
    ConfigureEnable { settle_ticks_remaining: u16 },
}

#[cfg(target_os = "none")]
impl DriverDebugService {
    #[cold]
    #[inline(never)]
    fn service(
        &mut self,
        driver: &MotorDriverPins,
        driver_spi: &Drv8323s,
        command_sequence: &mut u8,
        report_sequence: &mut u8,
    ) -> Option<Drv8323sConfigReport> {
        if let Some(packet) = obot_g474::usb::poll_driver_command()
            .or_else(|| debug_report::poll_driver_command(command_sequence))
        {
            return match packet.command {
                DriverCommand::Disable => {
                    self.pending = None;
                    driver.disable();
                    let report = Drv8323sConfigReport::default();
                    self.publish_report(report_sequence, report);
                    core::hint::black_box((packet.command, report.configured()));
                    Some(report)
                }
                DriverCommand::ConfigureEnable => {
                    driver.enable();
                    self.pending = Some(PendingDriverAction::ConfigureEnable {
                        settle_ticks_remaining: DRIVER_ENABLE_SETTLE_MAIN_TICKS,
                    });
                    core::hint::black_box(packet.command);
                    None
                }
            };
        }

        self.service_pending(driver, driver_spi, report_sequence)
    }

    fn service_pending(
        &mut self,
        driver: &MotorDriverPins,
        driver_spi: &Drv8323s,
        report_sequence: &mut u8,
    ) -> Option<Drv8323sConfigReport> {
        match self.pending.as_mut()? {
            PendingDriverAction::ConfigureEnable {
                settle_ticks_remaining,
            } if *settle_ticks_remaining > 0 => {
                *settle_ticks_remaining -= 1;
                None
            }
            PendingDriverAction::ConfigureEnable { .. } => {
                self.pending = None;
                let report = driver_spi.configure_motor_hall_registers();
                if !report.configured() {
                    driver.disable();
                }
                self.publish_report(report_sequence, report);
                Some(report)
            }
        }
    }

    fn publish_report(&self, report_sequence: &mut u8, report: Drv8323sConfigReport) {
        publish_driver_report(*report_sequence, report);
        *report_sequence = (*report_sequence).wrapping_add(1);
        core::hint::black_box(report.configured());
    }
}

#[cfg(target_os = "none")]
fn publish_driver_report(sequence: u8, report: Drv8323sConfigReport) {
    obot_g474::usb::publish_driver_report(report);
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

#[cfg(target_os = "none")]
fn controller_storage_mut() -> &'static mut Controller {
    // SAFETY: The current firmware is single-threaded at this layer: command
    // polling/status publication happen from the main-loop branch only, and no
    // interrupt handler accesses this controller storage. Keeping it out of
    // `firmware_main` avoids perturbing the measured 50 kHz fast-loop frame.
    unsafe { &mut *CONTROLLER.0.get() }
}

#[cfg(target_os = "none")]
fn apply_host_command(
    controller: &mut Controller,
    command: obot_core::MotorCommand,
) -> (bool, bool) {
    let mode = command.mode;
    let command_accepted = controller.apply(command).is_ok();
    let command_allows_output = command_accepted && mode_allows_output(mode);
    let clear_faults_accepted = command_accepted && mode == ControlMode::ClearFaults;
    core::hint::black_box((
        command_accepted,
        command_allows_output,
        clear_faults_accepted,
    ));
    (command_allows_output, clear_faults_accepted)
}

#[cfg(any(target_os = "none", test))]
fn mode_allows_output(mode: ControlMode) -> bool {
    matches!(
        mode,
        ControlMode::Torque | ControlMode::Velocity | ControlMode::Position
    )
}

#[cfg(target_os = "none")]
fn publish_status_report(sequence: u8, state: obot_core::MotorState) {
    let packet = StatusPacket { sequence, state };
    debug_report::publish_status(packet);
    obot_g474::usb::publish_realtime_status(packet);
    core::hint::black_box((
        debug_report::status_packet_ptr(),
        debug_report::status_packet_len(),
    ));
}

#[cfg(target_os = "none")]
fn publish_output_safety_report(sequence: u8, status: OutputSafetyStatus) -> u8 {
    obot_g474::usb::publish_output_safety_status(status);
    debug_report::publish_output_safety(OutputSafetyPacket { sequence, status });
    core::hint::black_box((
        debug_report::output_safety_packet_ptr(),
        debug_report::output_safety_packet_len(),
    ));
    sequence.wrapping_add(1)
}

#[cfg(target_os = "none")]
fn publish_bus_voltage_report(sequence: u8, raw: u16) -> u8 {
    obot_g474::usb::publish_bus_voltage_raw(raw);
    debug_report::publish_bus_voltage(BusVoltagePacket { sequence, raw });
    core::hint::black_box((
        debug_report::bus_voltage_packet_ptr(),
        debug_report::bus_voltage_packet_len(),
    ));
    sequence.wrapping_add(1)
}

#[cfg(target_os = "none")]
#[inline(never)]
fn update_output_safety(
    driver: &MotorDriverPins,
    command_allows_output: bool,
    bus_allows_output: bool,
    controller_faulted: bool,
    host_timed_out: bool,
    clear_latched_faults: bool,
) -> OutputSafetyStatus {
    let safety = output_safety_storage_mut();
    if clear_latched_faults {
        safety.clear_latched_driver_fault();
    }

    let driver_status = driver.status();
    let status = safety.update(OutputSafetyInputs {
        command_allows_output,
        bus_allows_output,
        driver_enabled: driver_status.enabled,
        driver_faulted: driver_status.faulted,
        controller_faulted,
        host_timed_out,
    });
    core::hint::black_box(status);
    status
}

#[cfg(target_os = "none")]
fn output_safety_storage_mut() -> &'static mut OutputSafety {
    // SAFETY: Output safety is updated only from the cooperative main-loop branch.
    unsafe { &mut *OUTPUT_SAFETY.0.get() }
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
fn run_preemptible_main_loop(
    benchmark: &mut LoopBenchmark,
    cycle_counter: &impl CycleCounter,
    work: impl FnOnce(),
) {
    let (sample, fast_cycles_before, usb_interrupts_before) = interrupt_free(|| {
        (
            benchmark.start(cycle_counter.now()),
            fast_loop_execution_total_cycles(),
            obot_g474::usb::interrupt_count(),
        )
    });
    work();
    let (now_cycles, fast_cycles_after, usb_interrupts_after) = interrupt_free(|| {
        (
            cycle_counter.now(),
            fast_loop_execution_total_cycles(),
            obot_g474::usb::interrupt_count(),
        )
    });
    if usb_interrupts_after != usb_interrupts_before {
        return;
    }
    let elapsed_cycles = now_cycles.wrapping_sub(sample.start_cycles());
    let fast_cycles = fast_cycles_after.saturating_sub(fast_cycles_before);
    let exclusive_cycles =
        elapsed_cycles.saturating_sub(fast_cycles.min(elapsed_cycles as u64) as u32);
    benchmark.finish_cycles(sample, exclusive_cycles);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_closed_loop_modes_allow_output() {
        assert!(!mode_allows_output(ControlMode::Disabled));
        assert!(mode_allows_output(ControlMode::Torque));
        assert!(mode_allows_output(ControlMode::Velocity));
        assert!(mode_allows_output(ControlMode::Position));
        assert!(!mode_allows_output(ControlMode::ClearFaults));
    }
}
