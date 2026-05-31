use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use obot_protocol::usb_control::{self, ControlRequest, ControlResponse, SetupPacket};

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

const SERIAL_STRING: &str = "000000000000";
const CONFIGURATION_STRING: &str = "rust firmware";
const INTERFACE_STRING: &str = "rust_debug";

static USB_LP_INTERRUPT_COUNT: AtomicU32 = AtomicU32::new(0);
static USB_ERROR_COUNT: AtomicU32 = AtomicU32::new(0);
static PENDING_ADDRESS: AtomicU8 = AtomicU8::new(NO_PENDING_ADDRESS);

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

pub fn interrupt_count() -> u32 {
    USB_LP_INTERRUPT_COUNT.load(Ordering::Relaxed)
}

pub fn error_count() -> u32 {
    USB_ERROR_COUNT.load(Ordering::Relaxed)
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
    if istr & USB_ISTR_EP_ID != 0 {
        return;
    }

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
    let Ok(response) = usb_control::control_response(
        request,
        SERIAL_STRING,
        CONFIGURATION_STRING,
        INTERFACE_STRING,
    ) else {
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

fn send_stall(endpoint: u8) {
    endpoint_set_toggle(endpoint, USB_EP_TX_STALL, USB_EPTX_STAT);
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

fn read16(address: usize) -> u16 {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { read_volatile(address as *const u16) }
}

fn write16(address: usize, value: u16) {
    // SAFETY: The caller passes STM32G474 memory-mapped register addresses.
    unsafe { write_volatile(address as *mut u16, value) };
}
