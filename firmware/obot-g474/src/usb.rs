use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use obot_core::{benchmark::BenchmarkReport, output::OutputSafetyStatus};
use obot_protocol::{
    COMMAND_PACKET_LEN, CommandPacket, DRIVER_COMMAND_PACKET_LEN, DriverCommandPacket,
    STATUS_PACKET_LEN, StatusPacket,
    usb_control::{self, ControlRequest, ControlResponse, SetupPacket},
};

use crate::drv8323s::Drv8323sConfigReport;

const RCC_BASE: usize = 0x4002_1000;
const RCC_APB1RSTR1: usize = RCC_BASE + 0x38;
const RCC_AHB2ENR: usize = RCC_BASE + 0x4C;
const RCC_APB1ENR1: usize = RCC_BASE + 0x58;

const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_APB1RSTR1_USBRST: u32 = 1 << 23;
const RCC_APB1ENR1_USBEN: u32 = 1 << 23;

const GPIOA_BASE: usize = 0x4800_0000;
const GPIO_MODER: usize = GPIOA_BASE;
const GPIO_OSPEEDR: usize = GPIOA_BASE + 0x08;
const GPIO_PUPDR: usize = GPIOA_BASE + 0x0C;

const UID_BASE: usize = 0x1FFF_7590;
const UID_WORD0: usize = UID_BASE;
const UID_WORD1: usize = UID_BASE + 4;
const UID_WORD2: usize = UID_BASE + 8;

const NVIC_ISER0: usize = 0xE000_E100;
const NVIC_IPR_BASE: usize = 0xE000_E400;
const USB_LP_IRQ: u8 = 20;
const USB_LP_IRQ_PRIORITY: u8 = 0x80;

const USB_BASE: usize = 0x4000_5C00;
const USB_EP0R: usize = USB_BASE;
const USB_CNTR: usize = USB_BASE + 0x40;
const USB_ISTR: usize = USB_BASE + 0x44;
const USB_DADDR: usize = USB_BASE + 0x4C;
const USB_BTABLE: usize = USB_BASE + 0x50;
const USB_BCDR: usize = USB_BASE + 0x58;

const USB_PMA_BASE: usize = 0x4000_6000;
const BTABLE_ENTRY_BYTES: usize = 8;
const BTABLE_ADDR_TX: usize = 0;
const BTABLE_COUNT_TX: usize = 2;
const BTABLE_ADDR_RX: usize = 4;
const BTABLE_COUNT_RX: usize = 6;
const EP_BUFFER_BYTES: u16 = 160;
const EP_TX_BYTES: u16 = 64;
const EP0_TX_OFFSET: u16 = 64;
const EP0_RX_OFFSET: u16 = EP0_TX_OFFSET + EP_TX_BYTES;
const EP1_TX_OFFSET: u16 = EP0_TX_OFFSET + EP_BUFFER_BYTES;
const EP1_RX_OFFSET: u16 = EP1_TX_OFFSET + EP_TX_BYTES;
const EP2_TX_OFFSET: u16 = EP1_TX_OFFSET + EP_BUFFER_BYTES;
const EP2_RX_OFFSET: u16 = EP2_TX_OFFSET + EP_TX_BYTES;

const USB_DM_PIN: u32 = 11;
const USB_DP_PIN: u32 = 12;
const USB_PIN_MASK: u32 = two_bit_pin_mask(USB_DM_PIN) | two_bit_pin_mask(USB_DP_PIN);

const GPIO_MODE_ANALOG: u32 = 0b11;
const GPIO_PULL_NONE: u32 = 0b00;

const USB_EP_CTR_RX: u16 = 0x8000;
const USB_EP_DTOG_RX: u16 = 0x4000;
const USB_EP_SETUP: u16 = 0x0800;
const USB_EP_T_FIELD: u16 = 0x0600;
const USB_EP_KIND: u16 = 0x0100;
const USB_EP_CTR_TX: u16 = 0x0080;
const USB_EP_DTOG_TX: u16 = 0x0040;
const USB_EPADDR_FIELD: u16 = 0x000F;
const USB_EPREG_MASK: u16 =
    USB_EP_CTR_RX | USB_EP_SETUP | USB_EP_T_FIELD | USB_EP_KIND | USB_EP_CTR_TX | USB_EPADDR_FIELD;
const USB_EP_CONTROL: u16 = 0x0200;
const USB_EP_BULK: u16 = 0x0000;
const USB_EPTX_STAT: u16 = 0x0030;
const USB_EP_TX_STALL: u16 = 0x0010;
const USB_EP_TX_NAK: u16 = 0x0020;
const USB_EP_TX_VALID: u16 = 0x0030;
const USB_EPRX_STAT: u16 = 0x3000;
const USB_EP_RX_VALID: u16 = 0x3000;

const USB_CNTR_L1REQM: u16 = 0x0080;
const USB_CNTR_RESETM: u16 = 0x0400;
const USB_CNTR_SUSPM: u16 = 0x0800;
const USB_CNTR_WKUPM: u16 = 0x1000;
const USB_CNTR_ERRM: u16 = 0x2000;
const USB_CNTR_CTRM: u16 = 0x8000;
const USB_CNTR_INITIAL_MASKS: u16 = USB_CNTR_L1REQM
    | USB_CNTR_RESETM
    | USB_CNTR_SUSPM
    | USB_CNTR_WKUPM
    | USB_CNTR_ERRM
    | USB_CNTR_CTRM;

const USB_ISTR_EP_ID: u16 = 0x000F;
const USB_ISTR_DIR: u16 = 0x0010;
const USB_ISTR_ESOF: u16 = 0x0100;
const USB_ISTR_RESET: u16 = 0x0400;
const USB_ISTR_SUSP: u16 = 0x0800;
const USB_ISTR_ERR: u16 = 0x2000;
const USB_ISTR_CTR: u16 = 0x8000;
const USB_ISTR_CLEAR_ALL: u16 = 0;

const USB_DADDR_ADD: u16 = 0x007F;
const USB_DADDR_EF: u16 = 0x0080;
const USB_BCDR_DPPU: u16 = 0x8000;
const USB_COUNT_RX_COUNT_MASK: u16 = 0x03FF;
const USB_COUNT_RX_96_BYTES: u16 = (1 << 15) | (2 << 10);
const NO_PENDING_ADDRESS: u8 = 0xFF;

const CONFIGURATION_STRING: &str = "obot-rs rust firmware";
const INTERFACE_STRING: &str = "rust_debug";

static USB_LP_INTERRUPT_COUNT: AtomicU32 = AtomicU32::new(0);
static USB_ERROR_COUNT: AtomicU32 = AtomicU32::new(0);
static PENDING_ADDRESS: AtomicU8 = AtomicU8::new(NO_PENDING_ADDRESS);
static TEXT_RX_LAST_LEN: AtomicU8 = AtomicU8::new(0);
static TEXT_TX_LAST_LEN: AtomicU8 = AtomicU8::new(0);
static TEXT_RX_TOTAL: AtomicU32 = AtomicU32::new(0);
static TEXT_RX_UNSUPPORTED: AtomicU32 = AtomicU32::new(0);
static TEXT_TX_TOTAL: AtomicU32 = AtomicU32::new(0);
static TEXT_TX_BUSY: AtomicU32 = AtomicU32::new(0);
static REALTIME_COMMAND_VERSION: AtomicU32 = AtomicU32::new(0);
static REALTIME_COMMAND_CONSUMED_VERSION: AtomicU32 = AtomicU32::new(0);
static DRIVER_COMMAND_VERSION: AtomicU32 = AtomicU32::new(0);
static DRIVER_COMMAND_CONSUMED_VERSION: AtomicU32 = AtomicU32::new(0);
static REALTIME_RX_LAST_LEN: AtomicU8 = AtomicU8::new(0);
static REALTIME_RX_TOTAL: AtomicU32 = AtomicU32::new(0);
static REALTIME_RX_ACCEPTED: AtomicU32 = AtomicU32::new(0);
static REALTIME_RX_UNSUPPORTED: AtomicU32 = AtomicU32::new(0);
static REALTIME_TX_TOTAL: AtomicU32 = AtomicU32::new(0);
static REALTIME_TX_BUSY: AtomicU32 = AtomicU32::new(0);
static REALTIME_COMMAND_BYTES: [AtomicU8; COMMAND_PACKET_LEN] =
    [const { AtomicU8::new(0) }; COMMAND_PACKET_LEN];
static DRIVER_COMMAND_BYTES: [AtomicU8; DRIVER_COMMAND_PACKET_LEN] =
    [const { AtomicU8::new(0) }; DRIVER_COMMAND_PACKET_LEN];
static REALTIME_STATUS_BYTES: [AtomicU8; STATUS_PACKET_LEN] =
    [const { AtomicU8::new(0) }; STATUS_PACKET_LEN];
static BENCH_T_EXEC_FASTLOOP: AtomicU32 = AtomicU32::new(0);
static BENCH_T_PERIOD_FASTLOOP: AtomicU32 = AtomicU32::new(0);
static BENCH_T_EXEC_MAINLOOP: AtomicU32 = AtomicU32::new(0);
static BENCH_T_PERIOD_MAINLOOP: AtomicU32 = AtomicU32::new(0);
static OUTPUT_SAFETY_FLAGS: AtomicU32 = AtomicU32::new(0);
static BUS_VOLTAGE_RAW: AtomicU32 = AtomicU32::new(0);
static DRIVER_CONFIGURED: AtomicU8 = AtomicU8::new(0);
static DRIVER_VERIFY_ERROR_MASK: AtomicU32 = AtomicU32::new(0);
static DRIVER_TRANSFER_ERROR_MASK: AtomicU32 = AtomicU32::new(0);
static HRTIM_OUTPUT_DISABLE_STATUS: AtomicU32 = AtomicU32::new(0);
static HRTIM_BRIDGE_OUTPUT_FLAGS: AtomicU32 = AtomicU32::new(BRIDGE_OUTPUTS_DISABLED_BIT);
static BRIDGE_PREARM_BLOCKERS: AtomicU32 = AtomicU32::new(BRIDGE_PREARM_NOT_PUBLISHED_BIT);
static DRIVER_STATUS_BEFORE: AtomicU32 = AtomicU32::new(0);
static DRIVER_STATUS_AFTER: AtomicU32 = AtomicU32::new(0);

const OUTPUT_ALLOWED_BIT: u32 = 1 << 0;
const COMMAND_BLOCKED_BIT: u32 = 1 << 1;
const BUS_BLOCKED_BIT: u32 = 1 << 2;
const DRIVER_NOT_ENABLED_BIT: u32 = 1 << 3;
const DRIVER_FAULT_LATCHED_BIT: u32 = 1 << 4;
const CONTROLLER_FAULTED_BIT: u32 = 1 << 5;
const HOST_TIMED_OUT_BIT: u32 = 1 << 6;
const BRIDGE_OUTPUTS_DISABLED_BIT: u32 = 1 << 0;
const BRIDGE_OUTPUTS_ENABLED_BIT: u32 = 1 << 1;
const BRIDGE_PREARM_NOT_PUBLISHED_BIT: u32 = 1 << 31;

pub struct UsbDevice;

impl UsbDevice {
    pub fn prepare_disconnected() -> Self {
        enable_gpioa_clock();
        configure_usb_pins();
        enable_usb_clock();
        reset_usb();
        configure_usb_disconnected();
        Self
    }

    pub fn connect(&self) {
        enable_usb_lp_interrupt();
        modify16(USB_BCDR, |value| value | USB_BCDR_DPPU);
    }
}

pub fn interrupt() {
    USB_LP_INTERRUPT_COUNT.fetch_add(1, Ordering::Relaxed);

    let istr = read16(USB_ISTR);
    if istr & USB_ISTR_RESET != 0 {
        handle_reset();
    }
    if istr & USB_ISTR_SUSP != 0 {
        clear_istr(USB_ISTR_SUSP);
    }
    if istr & USB_ISTR_CTR != 0 {
        handle_correct_transfer();
    }
    if read16(USB_ISTR) & (USB_ISTR_ERR | USB_ISTR_ESOF) != 0 {
        USB_ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
        clear_istr(USB_ISTR_ERR | USB_ISTR_ESOF);
    }
}

pub fn poll_realtime_command() -> Option<CommandPacket> {
    poll_versioned_packet::<COMMAND_PACKET_LEN, CommandPacket>(
        &REALTIME_COMMAND_VERSION,
        &REALTIME_COMMAND_CONSUMED_VERSION,
        &REALTIME_COMMAND_BYTES,
        CommandPacket::decode,
    )
}

pub fn poll_driver_command() -> Option<DriverCommandPacket> {
    poll_versioned_packet::<DRIVER_COMMAND_PACKET_LEN, DriverCommandPacket>(
        &DRIVER_COMMAND_VERSION,
        &DRIVER_COMMAND_CONSUMED_VERSION,
        &DRIVER_COMMAND_BYTES,
        DriverCommandPacket::decode,
    )
}

fn poll_versioned_packet<const N: usize, T>(
    version_storage: &AtomicU32,
    consumed_storage: &AtomicU32,
    packet_storage: &[AtomicU8; N],
    decode: impl Fn(&[u8]) -> Result<T, obot_protocol::DecodeError>,
) -> Option<T> {
    let consumed = consumed_storage.load(Ordering::Relaxed);
    let mut version = version_storage.load(Ordering::Acquire);
    if version == consumed || version & 1 != 0 {
        return None;
    }

    for _ in 0..3 {
        let mut bytes = [0; N];
        for (byte, storage) in bytes.iter_mut().zip(packet_storage.iter()) {
            *byte = storage.load(Ordering::Relaxed);
        }
        let check = version_storage.load(Ordering::Acquire);
        if check == version {
            consumed_storage.store(version, Ordering::Release);
            return decode(&bytes).ok();
        }

        version = check;
        if version == consumed || version & 1 != 0 {
            return None;
        }
    }

    None
}

pub fn publish_realtime_status(packet: StatusPacket) {
    let encoded = packet.encode();
    for (storage, byte) in REALTIME_STATUS_BYTES.iter().zip(encoded) {
        storage.store(byte, Ordering::Relaxed);
    }
}

pub fn publish_text_api_benchmark(report: BenchmarkReport) {
    BENCH_T_EXEC_FASTLOOP.store(report.t_exec_fastloop(), Ordering::Relaxed);
    BENCH_T_PERIOD_FASTLOOP.store(report.t_period_fastloop(), Ordering::Relaxed);
    BENCH_T_EXEC_MAINLOOP.store(report.t_exec_mainloop(), Ordering::Relaxed);
    BENCH_T_PERIOD_MAINLOOP.store(report.t_period_mainloop(), Ordering::Relaxed);
}

pub fn publish_output_safety_status(status: OutputSafetyStatus) {
    OUTPUT_SAFETY_FLAGS.store(output_safety_flags(status), Ordering::Relaxed);
}

pub fn publish_bus_voltage_raw(raw: u16) {
    BUS_VOLTAGE_RAW.store(raw as u32, Ordering::Relaxed);
}

pub fn publish_hrtim_output_status(
    disable_status: u32,
    bridge_outputs_disabled: bool,
    bridge_outputs_enabled: bool,
) {
    HRTIM_OUTPUT_DISABLE_STATUS.store(disable_status, Ordering::Relaxed);
    HRTIM_BRIDGE_OUTPUT_FLAGS.store(
        bool_flag(bridge_outputs_disabled, BRIDGE_OUTPUTS_DISABLED_BIT)
            | bool_flag(bridge_outputs_enabled, BRIDGE_OUTPUTS_ENABLED_BIT),
        Ordering::Relaxed,
    );
}

pub fn publish_bridge_prearm_status(blockers: u32) {
    BRIDGE_PREARM_BLOCKERS.store(blockers, Ordering::Relaxed);
}

pub fn publish_driver_report(report: Drv8323sConfigReport) {
    DRIVER_CONFIGURED.store(u8::from(report.configured()), Ordering::Relaxed);
    DRIVER_VERIFY_ERROR_MASK.store(report.verify_error_mask as u32, Ordering::Relaxed);
    DRIVER_TRANSFER_ERROR_MASK.store(report.transfer_error_mask as u32, Ordering::Relaxed);
    DRIVER_STATUS_BEFORE.store(
        report.status_before.map_or(0, |status| status.as_u32()),
        Ordering::Relaxed,
    );
    DRIVER_STATUS_AFTER.store(
        report.status_after.map_or(0, |status| status.as_u32()),
        Ordering::Relaxed,
    );
}

fn output_safety_flags(status: OutputSafetyStatus) -> u32 {
    bool_flag(status.output_allowed, OUTPUT_ALLOWED_BIT)
        | bool_flag(status.command_blocked, COMMAND_BLOCKED_BIT)
        | bool_flag(status.bus_blocked, BUS_BLOCKED_BIT)
        | bool_flag(status.driver_not_enabled, DRIVER_NOT_ENABLED_BIT)
        | bool_flag(status.driver_fault_latched, DRIVER_FAULT_LATCHED_BIT)
        | bool_flag(status.controller_faulted, CONTROLLER_FAULTED_BIT)
        | bool_flag(status.host_timed_out, HOST_TIMED_OUT_BIT)
}

fn bool_flag(value: bool, bit: u32) -> u32 {
    if value { bit } else { 0 }
}

fn enable_gpioa_clock() {
    modify32(RCC_AHB2ENR, |value| value | RCC_AHB2ENR_GPIOAEN);
    let _ = read32(RCC_AHB2ENR);
}

fn configure_usb_pins() {
    modify32(GPIO_MODER, |value| {
        set_two_bit_field(
            set_two_bit_field(value, USB_DM_PIN, GPIO_MODE_ANALOG),
            USB_DP_PIN,
            GPIO_MODE_ANALOG,
        )
    });
    modify32(GPIO_OSPEEDR, |value| value & !USB_PIN_MASK);
    modify32(GPIO_PUPDR, |value| {
        set_two_bit_field(
            set_two_bit_field(value, USB_DM_PIN, GPIO_PULL_NONE),
            USB_DP_PIN,
            GPIO_PULL_NONE,
        )
    });
}

fn enable_usb_clock() {
    modify32(RCC_APB1ENR1, |value| value | RCC_APB1ENR1_USBEN);
    let _ = read32(RCC_APB1ENR1);
}

fn reset_usb() {
    modify32(RCC_APB1RSTR1, |value| value | RCC_APB1RSTR1_USBRST);
    modify32(RCC_APB1RSTR1, |value| value & !RCC_APB1RSTR1_USBRST);
}

fn configure_usb_disconnected() {
    modify16(USB_BCDR, |value| value & !USB_BCDR_DPPU);
    write16(USB_BTABLE, 0);
    write16(USB_DADDR, 0);
    write16(USB_ISTR, USB_ISTR_CLEAR_ALL);
    write16(USB_CNTR, USB_CNTR_INITIAL_MASKS);
}

fn handle_reset() {
    write16(endpoint_register(0), USB_EP_CONTROL);
    write_btable(0, BTABLE_ADDR_TX, EP0_TX_OFFSET);
    endpoint_set_toggle(0, USB_EP_TX_NAK, USB_EPTX_STAT | USB_EP_DTOG_TX);
    write_btable(0, BTABLE_ADDR_RX, EP0_RX_OFFSET);
    write_btable(0, BTABLE_COUNT_RX, USB_COUNT_RX_96_BYTES);
    endpoint_set_toggle(0, USB_EP_RX_VALID, USB_EPRX_STAT | USB_EP_DTOG_RX);
    write16(USB_DADDR, USB_DADDR_EF);
    PENDING_ADDRESS.store(NO_PENDING_ADDRESS, Ordering::Relaxed);
    clear_istr(USB_ISTR_RESET);
}

fn handle_correct_transfer() {
    let istr = read16(USB_ISTR);
    match (istr & USB_ISTR_EP_ID) as u8 {
        0 => handle_control_endpoint_transfer(istr),
        1 => handle_text_endpoint_transfer(istr),
        2 => handle_realtime_endpoint_transfer(istr),
        _ => {}
    }
}

fn handle_control_endpoint_transfer(istr: u16) {
    if istr & USB_ISTR_DIR != 0 {
        if read16(endpoint_register(0)) & USB_EP_SETUP != 0 {
            handle_ep0_setup();
        }
        clear_endpoint_ctr_rx(0);
        endpoint_set_toggle(0, USB_EP_RX_VALID, USB_EPRX_STAT);
    }

    if read16(endpoint_register(0)) & USB_EP_CTR_TX != 0 {
        apply_pending_address();
        clear_endpoint_ctr_tx(0);
    }
}

fn handle_text_endpoint_transfer(istr: u16) {
    if istr & USB_ISTR_DIR != 0 {
        clear_endpoint_ctr_rx(1);
        let byte_count = read_btable(1, BTABLE_COUNT_RX) & USB_COUNT_RX_COUNT_MASK;
        let len = core::cmp::min(
            byte_count as usize,
            usb_control::BULK_MAX_PACKET_SIZE as usize,
        );
        let mut request = [0; usb_control::BULK_MAX_PACKET_SIZE as usize];
        read_pma_bytes(EP1_RX_OFFSET, &mut request[..len], len);
        TEXT_RX_LAST_LEN.store(len as u8, Ordering::Relaxed);
        TEXT_RX_TOTAL.fetch_add(1, Ordering::Relaxed);
        endpoint_set_toggle(1, USB_EP_RX_VALID, USB_EPRX_STAT);
        send_text_api_response_immediate(&request[..len]);
    }

    if read16(endpoint_register(1)) & USB_EP_CTR_TX != 0 {
        clear_endpoint_ctr_tx(1);
        set_tx_nak(1);
    }
}

fn handle_realtime_endpoint_transfer(istr: u16) {
    if read16(endpoint_register(2)) & USB_EP_CTR_TX != 0 {
        clear_endpoint_ctr_tx(2);
    }

    if istr & USB_ISTR_DIR != 0 {
        clear_endpoint_ctr_rx(2);
        let byte_count = read_btable(2, BTABLE_COUNT_RX) & USB_COUNT_RX_COUNT_MASK;
        let len = core::cmp::min(
            byte_count as usize,
            usb_control::BULK_MAX_PACKET_SIZE as usize,
        );
        REALTIME_RX_LAST_LEN.store(len as u8, Ordering::Relaxed);
        REALTIME_RX_TOTAL.fetch_add(1, Ordering::Relaxed);
        let accepted = match len {
            COMMAND_PACKET_LEN => {
                accept_realtime_command_packet();
                true
            }
            DRIVER_COMMAND_PACKET_LEN => {
                accept_driver_command_packet();
                true
            }
            _ => {
                REALTIME_RX_UNSUPPORTED.fetch_add(1, Ordering::Relaxed);
                false
            }
        };
        endpoint_set_toggle(2, USB_EP_RX_VALID, USB_EPRX_STAT);
        if accepted {
            send_realtime_status_snapshot();
        }
    }
}

fn accept_realtime_command_packet() {
    let mut bytes = [0; COMMAND_PACKET_LEN];
    read_pma_bytes(EP2_RX_OFFSET, &mut bytes, COMMAND_PACKET_LEN);
    publish_versioned_packet(&REALTIME_COMMAND_VERSION, &REALTIME_COMMAND_BYTES, &bytes);
    REALTIME_RX_ACCEPTED.fetch_add(1, Ordering::Relaxed);
}

fn accept_driver_command_packet() {
    let mut bytes = [0; DRIVER_COMMAND_PACKET_LEN];
    read_pma_bytes(EP2_RX_OFFSET, &mut bytes, DRIVER_COMMAND_PACKET_LEN);
    publish_versioned_packet(&DRIVER_COMMAND_VERSION, &DRIVER_COMMAND_BYTES, &bytes);
    REALTIME_RX_ACCEPTED.fetch_add(1, Ordering::Relaxed);
}

fn publish_versioned_packet<const N: usize>(
    version_storage: &AtomicU32,
    packet_storage: &[AtomicU8; N],
    bytes: &[u8; N],
) {
    let version = version_storage.load(Ordering::Relaxed);
    version_storage.store(version.wrapping_add(1) | 1, Ordering::Release);
    for (storage, byte) in packet_storage.iter().zip(bytes) {
        storage.store(*byte, Ordering::Relaxed);
    }
    version_storage.store(version.wrapping_add(2) & !1, Ordering::Release);
}

fn send_realtime_status_snapshot() {
    if tx_active(2) {
        REALTIME_TX_BUSY.fetch_add(1, Ordering::Relaxed);
        set_tx_nak(2);
    }

    let mut encoded = [0; STATUS_PACKET_LEN];
    for (byte, storage) in encoded.iter_mut().zip(REALTIME_STATUS_BYTES.iter()) {
        *byte = storage.load(Ordering::Relaxed);
    }
    send_data(2, &encoded);
    REALTIME_TX_TOTAL.fetch_add(1, Ordering::Relaxed);
}

const USB_TEXT_API_NAMES: &[&str] = &[
    "api_length",
    "api_name",
    "cpu_frequency",
    "messages_version",
    "t_exec_fastloop",
    "t_period_fastloop",
    "t_exec_mainloop",
    "t_period_mainloop",
    "output_allowed",
    "command_blocked",
    "bus_blocked",
    "driver_not_enabled",
    "driver_fault_latched",
    "controller_faulted",
    "host_timed_out",
    "bus_voltage_raw",
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
    "realtime_rx_last_len",
    "realtime_rx_total",
    "realtime_rx_accepted",
    "realtime_rx_unsupported",
    "realtime_command_version",
    "realtime_command_consumed_version",
    "driver_command_version",
    "driver_command_consumed_version",
];

fn send_text_api_response_immediate(request: &[u8]) {
    let mut response = [0; usb_control::BULK_MAX_PACKET_SIZE as usize];
    let Some(response_len) = format_text_api_response(request, &mut response) else {
        TEXT_RX_UNSUPPORTED.fetch_add(1, Ordering::Relaxed);
        send_text_api_packet(&[]);
        return;
    };

    send_text_api_packet(&response[..response_len]);
}

fn send_text_api_packet(data: &[u8]) {
    if tx_active(1) {
        TEXT_TX_BUSY.fetch_add(1, Ordering::Relaxed);
        set_tx_nak(1);
    }
    send_data(1, data);
    TEXT_TX_LAST_LEN.store(data.len() as u8, Ordering::Relaxed);
    TEXT_TX_TOTAL.fetch_add(1, Ordering::Relaxed);
}

fn format_text_api_response(request: &[u8], output: &mut [u8]) -> Option<usize> {
    if request == b"api_length" {
        return write_u32_decimal(USB_TEXT_API_NAMES.len() as u32, output);
    }
    if let Some(index_bytes) = request.strip_prefix(b"api_name=") {
        let index = parse_decimal_usize(index_bytes)?;
        let name = USB_TEXT_API_NAMES.get(index)?;
        return write_bytes(name.as_bytes(), output);
    }

    match request {
        b"cpu_frequency" => write_u32_decimal(170_000_000, output),
        b"messages_version" => write_bytes(b"3.3", output),
        b"t_exec_fastloop" => {
            write_u32_decimal(BENCH_T_EXEC_FASTLOOP.load(Ordering::Relaxed), output)
        }
        b"t_period_fastloop" => {
            write_u32_decimal(BENCH_T_PERIOD_FASTLOOP.load(Ordering::Relaxed), output)
        }
        b"t_exec_mainloop" => {
            write_u32_decimal(BENCH_T_EXEC_MAINLOOP.load(Ordering::Relaxed), output)
        }
        b"t_period_mainloop" => {
            write_u32_decimal(BENCH_T_PERIOD_MAINLOOP.load(Ordering::Relaxed), output)
        }
        b"output_allowed" => write_bool(load_output_safety_flag(OUTPUT_ALLOWED_BIT), output),
        b"command_blocked" => write_bool(load_output_safety_flag(COMMAND_BLOCKED_BIT), output),
        b"bus_blocked" => write_bool(load_output_safety_flag(BUS_BLOCKED_BIT), output),
        b"driver_not_enabled" => {
            write_bool(load_output_safety_flag(DRIVER_NOT_ENABLED_BIT), output)
        }
        b"driver_fault_latched" => {
            write_bool(load_output_safety_flag(DRIVER_FAULT_LATCHED_BIT), output)
        }
        b"controller_faulted" => {
            write_bool(load_output_safety_flag(CONTROLLER_FAULTED_BIT), output)
        }
        b"host_timed_out" => write_bool(load_output_safety_flag(HOST_TIMED_OUT_BIT), output),
        b"bus_voltage_raw" => write_u32_decimal(BUS_VOLTAGE_RAW.load(Ordering::Relaxed), output),
        b"bridge_output_disable_status" => {
            write_u32_decimal(HRTIM_OUTPUT_DISABLE_STATUS.load(Ordering::Relaxed), output)
        }
        b"bridge_outputs_disabled" => write_bool(bridge_outputs_disabled(), output),
        b"bridge_outputs_enabled" => write_bool(bridge_outputs_enabled(), output),
        b"bridge_prearm_ready" => write_bool(bridge_prearm_ready(), output),
        b"bridge_prearm_blockers" => {
            write_u32_decimal(BRIDGE_PREARM_BLOCKERS.load(Ordering::Relaxed), output)
        }
        b"driver_configured" => write_bool(DRIVER_CONFIGURED.load(Ordering::Relaxed) != 0, output),
        b"verify_error_mask" => {
            write_u32_decimal(DRIVER_VERIFY_ERROR_MASK.load(Ordering::Relaxed), output)
        }
        b"transfer_error_mask" => {
            write_u32_decimal(DRIVER_TRANSFER_ERROR_MASK.load(Ordering::Relaxed), output)
        }
        b"status_before" => write_u32_decimal(DRIVER_STATUS_BEFORE.load(Ordering::Relaxed), output),
        b"status_after" => write_u32_decimal(DRIVER_STATUS_AFTER.load(Ordering::Relaxed), output),
        b"realtime_rx_last_len" => {
            write_u32_decimal(REALTIME_RX_LAST_LEN.load(Ordering::Relaxed) as u32, output)
        }
        b"realtime_rx_total" => {
            write_u32_decimal(REALTIME_RX_TOTAL.load(Ordering::Relaxed), output)
        }
        b"realtime_rx_accepted" => {
            write_u32_decimal(REALTIME_RX_ACCEPTED.load(Ordering::Relaxed), output)
        }
        b"realtime_rx_unsupported" => {
            write_u32_decimal(REALTIME_RX_UNSUPPORTED.load(Ordering::Relaxed), output)
        }
        b"realtime_command_version" => {
            write_u32_decimal(REALTIME_COMMAND_VERSION.load(Ordering::Relaxed), output)
        }
        b"realtime_command_consumed_version" => write_u32_decimal(
            REALTIME_COMMAND_CONSUMED_VERSION.load(Ordering::Relaxed),
            output,
        ),
        b"driver_command_version" => {
            write_u32_decimal(DRIVER_COMMAND_VERSION.load(Ordering::Relaxed), output)
        }
        b"driver_command_consumed_version" => write_u32_decimal(
            DRIVER_COMMAND_CONSUMED_VERSION.load(Ordering::Relaxed),
            output,
        ),
        _ => None,
    }
}

fn load_output_safety_flag(bit: u32) -> bool {
    OUTPUT_SAFETY_FLAGS.load(Ordering::Relaxed) & bit != 0
}

fn bridge_outputs_disabled() -> bool {
    HRTIM_BRIDGE_OUTPUT_FLAGS.load(Ordering::Relaxed) & BRIDGE_OUTPUTS_DISABLED_BIT != 0
}

fn bridge_outputs_enabled() -> bool {
    HRTIM_BRIDGE_OUTPUT_FLAGS.load(Ordering::Relaxed) & BRIDGE_OUTPUTS_ENABLED_BIT != 0
}

fn bridge_prearm_ready() -> bool {
    BRIDGE_PREARM_BLOCKERS.load(Ordering::Relaxed) == 0
}

fn parse_decimal_usize(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut value = 0_usize;
    for &byte in input {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
    }
    Some(value)
}

fn write_bool(value: bool, output: &mut [u8]) -> Option<usize> {
    write_bytes(if value { b"true" } else { b"false" }, output)
}

fn write_u32_decimal(value: u32, output: &mut [u8]) -> Option<usize> {
    let mut scratch = [0; 10];
    let mut value = value;
    let mut index = scratch.len();
    loop {
        index -= 1;
        scratch[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    write_bytes(&scratch[index..], output)
}

fn write_bytes(bytes: &[u8], output: &mut [u8]) -> Option<usize> {
    if bytes.len() > output.len() {
        return None;
    }
    output[..bytes.len()].copy_from_slice(bytes);
    Some(bytes.len())
}

fn handle_ep0_setup() {
    let byte_count = read_btable(0, BTABLE_COUNT_RX) & USB_COUNT_RX_COUNT_MASK;
    let mut setup_bytes = [0; usb_control::SETUP_PACKET_LEN];
    read_pma_bytes(EP0_RX_OFFSET, &mut setup_bytes, byte_count as usize);

    let Ok(setup) = SetupPacket::decode(&setup_bytes) else {
        send_stall(0);
        return;
    };
    let Ok(request) = setup.control_request() else {
        send_stall(0);
        return;
    };

    match request {
        ControlRequest::SetAddress(address) => {
            PENDING_ADDRESS.store(address & USB_DADDR_ADD as u8, Ordering::Relaxed);
            send_data(0, &[]);
        }
        ControlRequest::SetConfiguration(_) => {
            configure_bulk_endpoints();
            send_data(0, &[]);
        }
        ControlRequest::SetInterface { .. } | ControlRequest::DfuDetach { .. } => {
            send_data(0, &[]);
        }
        _ => send_control_response(setup, request),
    }
}

fn send_control_response(setup: SetupPacket, request: ControlRequest) {
    let mut serial_buffer = [0; 16];
    let serial = serial_string(&mut serial_buffer);
    let Ok(response) =
        usb_control::control_response(request, serial, CONFIGURATION_STRING, INTERFACE_STRING)
    else {
        send_stall(0);
        return;
    };

    match response {
        ControlResponse::Bytes(bytes) => {
            send_data(0, usb_control::limit_to_setup_length(bytes, setup))
        }
        ControlResponse::StringAscii(value) => {
            let mut descriptor = [0; 64];
            let Ok(bytes) = usb_control::write_ascii_string_descriptor(value, &mut descriptor)
            else {
                send_stall(0);
                return;
            };
            send_data(0, usb_control::limit_to_setup_length(bytes, setup));
        }
        ControlResponse::StatusAck => send_data(0, &[]),
    }
}

fn configure_bulk_endpoints() {
    configure_bulk_endpoint(1, EP1_TX_OFFSET, EP1_RX_OFFSET);
    configure_bulk_endpoint(2, EP2_TX_OFFSET, EP2_RX_OFFSET);
}

fn configure_bulk_endpoint(endpoint: u8, tx_offset: u16, rx_offset: u16) {
    write16(endpoint_register(endpoint), endpoint as u16 | USB_EP_BULK);
    write_btable(endpoint, BTABLE_ADDR_TX, tx_offset);
    endpoint_set_toggle(endpoint, USB_EP_TX_NAK, USB_EPTX_STAT | USB_EP_DTOG_TX);
    write_btable(endpoint, BTABLE_ADDR_RX, rx_offset);
    write_btable(endpoint, BTABLE_COUNT_RX, USB_COUNT_RX_96_BYTES);
    endpoint_set_toggle(endpoint, USB_EP_RX_VALID, USB_EPRX_STAT | USB_EP_DTOG_RX);
}

fn send_data(endpoint: u8, data: &[u8]) {
    let length = core::cmp::min(data.len(), usb_control::BULK_MAX_PACKET_SIZE as usize);
    write_pma_bytes(tx_offset(endpoint), &data[..length]);
    write_btable(endpoint, BTABLE_COUNT_TX, length as u16);
    endpoint_set_toggle(endpoint, USB_EP_TX_VALID, USB_EPTX_STAT);
}

fn tx_active(endpoint: u8) -> bool {
    read16(endpoint_register(endpoint)) & USB_EPTX_STAT == USB_EP_TX_VALID
}

fn send_stall(endpoint: u8) {
    endpoint_set_toggle(endpoint, USB_EP_TX_STALL, USB_EPTX_STAT);
}

fn set_tx_nak(endpoint: u8) {
    endpoint_set_toggle(endpoint, USB_EP_TX_NAK, USB_EPTX_STAT);
}

fn apply_pending_address() {
    let address = PENDING_ADDRESS.swap(NO_PENDING_ADDRESS, Ordering::Relaxed);
    if address != NO_PENDING_ADDRESS {
        write16(USB_DADDR, USB_DADDR_EF | (address as u16 & USB_DADDR_ADD));
    }
}

fn clear_istr(mask: u16) {
    modify16(USB_ISTR, |value| value & !mask);
}

fn clear_endpoint_ctr_rx(endpoint: u8) {
    let value = read16(endpoint_register(endpoint));
    write16(
        endpoint_register(endpoint),
        (USB_EP_CTR_TX | (value & USB_EPREG_MASK)) & !USB_EP_CTR_RX,
    );
}

fn clear_endpoint_ctr_tx(endpoint: u8) {
    let value = read16(endpoint_register(endpoint));
    write16(
        endpoint_register(endpoint),
        (USB_EP_CTR_RX | (value & USB_EPREG_MASK)) & !USB_EP_CTR_TX,
    );
}

fn endpoint_set_toggle(endpoint: u8, set_bits: u16, set_mask: u16) {
    let value = read16(endpoint_register(endpoint));
    let toggle = (value & set_mask) ^ set_bits;
    let normal = value & USB_EPREG_MASK;
    write16(
        endpoint_register(endpoint),
        normal | toggle | USB_EP_CTR_TX | USB_EP_CTR_RX,
    );
}

fn write_btable(endpoint: u8, field_offset: usize, value: u16) {
    write_pma16(endpoint as usize * BTABLE_ENTRY_BYTES + field_offset, value);
}

fn read_btable(endpoint: u8, field_offset: usize) -> u16 {
    read_pma16(endpoint as usize * BTABLE_ENTRY_BYTES + field_offset)
}

fn tx_offset(endpoint: u8) -> u16 {
    read_btable(endpoint, BTABLE_ADDR_TX)
}

fn read_pma_bytes(offset: u16, output: &mut [u8], byte_count: usize) {
    let count = core::cmp::min(byte_count, output.len());
    let word_count = count.div_ceil(2);
    for word_index in 0..word_count {
        let word = read_pma16(offset as usize + word_index * 2);
        let byte_index = word_index * 2;
        output[byte_index] = word as u8;
        if byte_index + 1 < count {
            output[byte_index + 1] = (word >> 8) as u8;
        }
    }
}

fn write_pma_bytes(offset: u16, data: &[u8]) {
    for (word_index, chunk) in data.chunks(2).enumerate() {
        let low = chunk[0] as u16;
        let high = chunk.get(1).copied().unwrap_or(0) as u16;
        write_pma16(offset as usize + word_index * 2, low | (high << 8));
    }
}

fn endpoint_register(endpoint: u8) -> usize {
    USB_EP0R + endpoint as usize * 4
}

fn write_pma16(offset: usize, value: u16) {
    write16(USB_PMA_BASE + offset, value);
}

fn read_pma16(offset: usize) -> u16 {
    read16(USB_PMA_BASE + offset)
}

fn enable_usb_lp_interrupt() {
    write8(NVIC_IPR_BASE + USB_LP_IRQ as usize, USB_LP_IRQ_PRIORITY);
    write32(NVIC_ISER0, 1 << USB_LP_IRQ);
}

fn serial_string(buffer: &mut [u8; 16]) -> &str {
    let serial0 = read32(UID_WORD0).wrapping_add(read32(UID_WORD2));
    let serial1 = (read32(UID_WORD1) >> 16) as u16;
    let mut len = append_hex_u32(buffer, 0, serial0);
    len = append_hex_u16(buffer, len, serial1);
    core::str::from_utf8(&buffer[..len]).unwrap_or("000000000000")
}

fn append_hex_u32(buffer: &mut [u8], mut len: usize, value: u32) -> usize {
    if value == 0 {
        buffer[len] = b'0';
        return len + 1;
    }

    let mut started = false;
    for shift in (0..=28).rev().step_by(4) {
        let digit = ((value >> shift) & 0xF) as u8;
        if digit == 0 && !started {
            continue;
        }
        started = true;
        buffer[len] = hex_digit(digit);
        len += 1;
    }
    len
}

fn append_hex_u16(buffer: &mut [u8], mut len: usize, value: u16) -> usize {
    if value == 0 {
        buffer[len] = b'0';
        return len + 1;
    }

    let mut started = false;
    for shift in (0..=12).rev().step_by(4) {
        let digit = ((value >> shift) & 0xF) as u8;
        if digit == 0 && !started {
            continue;
        }
        started = true;
        buffer[len] = hex_digit(digit);
        len += 1;
    }
    len
}

fn hex_digit(digit: u8) -> u8 {
    match digit {
        0..=9 => b'0' + digit,
        _ => b'A' + (digit - 10),
    }
}

fn set_two_bit_field(value: u32, pin: u32, field: u32) -> u32 {
    let shift = pin * 2;
    (value & !(0b11 << shift)) | (field << shift)
}

const fn two_bit_pin_mask(pin: u32) -> u32 {
    0b11 << (pin * 2)
}

fn modify32(address: usize, f: impl FnOnce(u32) -> u32) {
    let value = read32(address);
    write32(address, f(value));
}

fn modify16(address: usize, f: impl FnOnce(u16) -> u16) {
    let value = read16(address);
    write16(address, f(value));
}

fn read32(address: usize) -> u32 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { read_volatile(address as *const u32) }
}

fn write32(address: usize, value: u32) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u32, value) };
}

fn write8(address: usize, value: u8) {
    // SAFETY: The caller passes ARMv7-M memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u8, value) };
}

fn read16(address: usize) -> u16 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { read_volatile(address as *const u16) }
}

fn write16(address: usize, value: u16) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u16, value) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immediate_text_api_exposes_run_stats_fields() {
        publish_text_api_benchmark(BenchmarkReport {
            fast: obot_core::benchmark::LoopBenchmarkSnapshot {
                period: obot_core::benchmark::CycleStatsSnapshot {
                    last_cycles: 3399,
                    ..obot_core::benchmark::CycleStatsSnapshot::default()
                },
                execution: obot_core::benchmark::CycleStatsSnapshot {
                    last_cycles: 305,
                    ..obot_core::benchmark::CycleStatsSnapshot::default()
                },
            },
            main: obot_core::benchmark::LoopBenchmarkSnapshot {
                period: obot_core::benchmark::CycleStatsSnapshot {
                    last_cycles: 16995,
                    ..obot_core::benchmark::CycleStatsSnapshot::default()
                },
                execution: obot_core::benchmark::CycleStatsSnapshot {
                    last_cycles: 1116,
                    ..obot_core::benchmark::CycleStatsSnapshot::default()
                },
            },
        });

        let mut output = [0; usb_control::BULK_MAX_PACKET_SIZE as usize];
        let len = format_text_api_response(b"api_length", &mut output).unwrap();
        assert_eq!(&output[..len], b"21");

        let len = format_text_api_response(b"api_name=4", &mut output).unwrap();
        assert_eq!(&output[..len], b"t_exec_fastloop");

        let len = format_text_api_response(b"t_exec_fastloop", &mut output).unwrap();
        assert_eq!(&output[..len], b"305");
        let len = format_text_api_response(b"t_period_fastloop", &mut output).unwrap();
        assert_eq!(&output[..len], b"3399");
        let len = format_text_api_response(b"t_exec_mainloop", &mut output).unwrap();
        assert_eq!(&output[..len], b"1116");
        let len = format_text_api_response(b"t_period_mainloop", &mut output).unwrap();
        assert_eq!(&output[..len], b"16995");

        assert_eq!(format_text_api_response(b"unknown", &mut output), None);
        assert_eq!(format_text_api_response(b"api_name=999", &mut output), None);
    }

    #[test]
    fn immediate_text_api_exposes_safety_and_driver_state() {
        publish_output_safety_status(OutputSafetyStatus {
            output_allowed: false,
            command_blocked: true,
            bus_blocked: true,
            driver_not_enabled: true,
            driver_fault_latched: false,
            controller_faulted: false,
            host_timed_out: true,
        });
        publish_bus_voltage_raw(1963);
        publish_driver_report(Drv8323sConfigReport {
            status_before: Some(crate::drv8323s::Drv8323sStatus {
                fault_status_1: 0x1234,
                vgs_status_2: 0x5678,
            }),
            status_after: Some(crate::drv8323s::Drv8323sStatus {
                fault_status_1: 0x00AA,
                vgs_status_2: 0x00BB,
            }),
            verify_error_mask: 0x12,
            transfer_error_mask: 0x40,
        });

        let mut output = [0; usb_control::BULK_MAX_PACKET_SIZE as usize];

        let len = format_text_api_response(b"output_allowed", &mut output).unwrap();
        assert_eq!(&output[..len], b"false");
        let len = format_text_api_response(b"command_blocked", &mut output).unwrap();
        assert_eq!(&output[..len], b"true");
        let len = format_text_api_response(b"bus_blocked", &mut output).unwrap();
        assert_eq!(&output[..len], b"true");
        let len = format_text_api_response(b"driver_not_enabled", &mut output).unwrap();
        assert_eq!(&output[..len], b"true");
        let len = format_text_api_response(b"driver_fault_latched", &mut output).unwrap();
        assert_eq!(&output[..len], b"false");
        let len = format_text_api_response(b"controller_faulted", &mut output).unwrap();
        assert_eq!(&output[..len], b"false");
        let len = format_text_api_response(b"host_timed_out", &mut output).unwrap();
        assert_eq!(&output[..len], b"true");
        let len = format_text_api_response(b"bus_voltage_raw", &mut output).unwrap();
        assert_eq!(&output[..len], b"1963");
        let len = format_text_api_response(b"driver_configured", &mut output).unwrap();
        assert_eq!(&output[..len], b"false");
        let len = format_text_api_response(b"verify_error_mask", &mut output).unwrap();
        assert_eq!(&output[..len], b"18");
        let len = format_text_api_response(b"transfer_error_mask", &mut output).unwrap();
        assert_eq!(&output[..len], b"64");
        let len = format_text_api_response(b"status_before", &mut output).unwrap();
        assert_eq!(&output[..len], b"1450709556");
        let len = format_text_api_response(b"status_after", &mut output).unwrap();
        assert_eq!(&output[..len], b"12255402");
    }
}
