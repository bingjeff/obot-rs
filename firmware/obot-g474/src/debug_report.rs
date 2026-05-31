use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use obot_protocol::{
    BENCHMARK_PACKET_LEN, BUS_VOLTAGE_PACKET_LEN, BenchmarkPacket, BusVoltagePacket,
    COMMAND_PACKET_LEN, CommandPacket, DRIVER_COMMAND_PACKET_LEN, DRIVER_REPORT_PACKET_LEN,
    DriverCommandPacket, DriverReportPacket, OUTPUT_SAFETY_PACKET_LEN, OutputSafetyPacket,
    STATUS_PACKET_LEN, StatusPacket, TEXT_API_REQUEST_PACKET_LEN, TEXT_API_RESPONSE_PACKET_LEN,
    TextApiRequestPacket, TextApiResponsePacket,
};

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BENCHMARK_PACKET: [u8; BENCHMARK_PACKET_LEN] = [0; BENCHMARK_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BENCHMARK_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_COMMAND_PACKET: [u8; COMMAND_PACKET_LEN] = [0; COMMAND_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_COMMAND_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_DRIVER_COMMAND_PACKET: [u8; DRIVER_COMMAND_PACKET_LEN] =
    [0; DRIVER_COMMAND_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_DRIVER_COMMAND_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_STATUS_PACKET: [u8; STATUS_PACKET_LEN] = [0; STATUS_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_STATUS_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_DRIVER_REPORT_PACKET: [u8; DRIVER_REPORT_PACKET_LEN] =
    [0; DRIVER_REPORT_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_DRIVER_REPORT_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_OUTPUT_SAFETY_PACKET: [u8; OUTPUT_SAFETY_PACKET_LEN] =
    [0; OUTPUT_SAFETY_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_OUTPUT_SAFETY_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BUS_VOLTAGE_PACKET: [u8; BUS_VOLTAGE_PACKET_LEN] = [0; BUS_VOLTAGE_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BUS_VOLTAGE_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_TEXT_API_REQUEST_PACKET: [u8; TEXT_API_REQUEST_PACKET_LEN] =
    [0; TEXT_API_REQUEST_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_TEXT_API_REQUEST_PACKET_SEQUENCE: u8 = 0;

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_TEXT_API_RESPONSE_PACKET: [u8; TEXT_API_RESPONSE_PACKET_LEN] =
    [0; TEXT_API_RESPONSE_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_TEXT_API_RESPONSE_PACKET_SEQUENCE: u8 = 0;

pub fn publish(packet: BenchmarkPacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_BENCHMARK_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported benchmark packet storage. Each
        // byte index is within the fixed packet length and is written exactly once.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the packet bytes, giving
    // debugger readers a cheap way to notice updates.
    unsafe {
        write_volatile(
            addr_of_mut!(OBOT_BENCHMARK_PACKET_SEQUENCE),
            packet.sequence,
        )
    };
}

pub fn poll_command(last_sequence: &mut u8) -> Option<CommandPacket> {
    let sequence = unsafe { read_volatile(addr_of!(OBOT_COMMAND_PACKET_SEQUENCE)) };
    if sequence == *last_sequence {
        return None;
    }

    let src = addr_of!(OBOT_COMMAND_PACKET).cast::<u8>();
    let mut bytes = [0; COMMAND_PACKET_LEN];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { read_volatile(src.add(offset)) };
    }

    match CommandPacket::decode(&bytes) {
        Ok(packet) if packet.sequence == sequence => {
            clear_command_sequence();
            *last_sequence = 0;
            Some(packet)
        }
        Ok(_) | Err(_) => {
            clear_command_sequence();
            *last_sequence = 0;
            None
        }
    }
}

pub fn poll_driver_command(last_sequence: &mut u8) -> Option<DriverCommandPacket> {
    let sequence = unsafe { read_volatile(addr_of!(OBOT_DRIVER_COMMAND_PACKET_SEQUENCE)) };
    if sequence == *last_sequence {
        return None;
    }

    let src = addr_of!(OBOT_DRIVER_COMMAND_PACKET).cast::<u8>();
    let mut bytes = [0; DRIVER_COMMAND_PACKET_LEN];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { read_volatile(src.add(offset)) };
    }

    match DriverCommandPacket::decode(&bytes) {
        Ok(packet) if packet.sequence == sequence => {
            clear_driver_command_sequence();
            *last_sequence = 0;
            Some(packet)
        }
        Ok(_) | Err(_) => {
            clear_driver_command_sequence();
            *last_sequence = 0;
            None
        }
    }
}

pub fn poll_text_api_request(last_sequence: &mut u8) -> Option<TextApiRequestPacket> {
    let sequence = unsafe { read_volatile(addr_of!(OBOT_TEXT_API_REQUEST_PACKET_SEQUENCE)) };
    if sequence == *last_sequence {
        return None;
    }

    let src = addr_of!(OBOT_TEXT_API_REQUEST_PACKET).cast::<u8>();
    let mut bytes = [0; TEXT_API_REQUEST_PACKET_LEN];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { read_volatile(src.add(offset)) };
    }

    match TextApiRequestPacket::decode(&bytes) {
        Ok(packet) if packet.sequence == sequence => {
            clear_text_api_request_sequence();
            *last_sequence = 0;
            Some(packet)
        }
        Ok(_) | Err(_) => {
            clear_text_api_request_sequence();
            *last_sequence = 0;
            None
        }
    }
}

fn clear_command_sequence() {
    unsafe { write_volatile(addr_of_mut!(OBOT_COMMAND_PACKET_SEQUENCE), 0) };
}

fn clear_driver_command_sequence() {
    unsafe { write_volatile(addr_of_mut!(OBOT_DRIVER_COMMAND_PACKET_SEQUENCE), 0) };
}

fn clear_text_api_request_sequence() {
    unsafe { write_volatile(addr_of_mut!(OBOT_TEXT_API_REQUEST_PACKET_SEQUENCE), 0) };
}

pub fn publish_status(packet: StatusPacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_STATUS_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported status packet storage. Each byte
        // index is within the fixed packet length and is written exactly once.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the status bytes.
    unsafe {
        write_volatile(addr_of_mut!(OBOT_STATUS_PACKET_SEQUENCE), packet.sequence);
    }
}

pub fn publish_driver_report(packet: DriverReportPacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_DRIVER_REPORT_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported driver report storage. Each byte
        // index is within the fixed packet length and is written exactly once.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the packet bytes.
    unsafe {
        write_volatile(
            addr_of_mut!(OBOT_DRIVER_REPORT_PACKET_SEQUENCE),
            packet.sequence,
        );
    }
}

pub fn publish_output_safety(packet: OutputSafetyPacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_OUTPUT_SAFETY_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported output-safety packet storage.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the packet bytes.
    unsafe {
        write_volatile(
            addr_of_mut!(OBOT_OUTPUT_SAFETY_PACKET_SEQUENCE),
            packet.sequence,
        );
    }
}

pub fn publish_bus_voltage(packet: BusVoltagePacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_BUS_VOLTAGE_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported bus-voltage packet storage.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the packet bytes.
    unsafe {
        write_volatile(
            addr_of_mut!(OBOT_BUS_VOLTAGE_PACKET_SEQUENCE),
            packet.sequence,
        );
    }
}

pub fn publish_text_api_response(packet: TextApiResponsePacket) {
    let encoded = packet.encode();
    let dest = addr_of_mut!(OBOT_TEXT_API_RESPONSE_PACKET).cast::<u8>();

    for (offset, byte) in encoded.iter().copied().enumerate() {
        // SAFETY: `dest` points to the exported text API response storage.
        unsafe { write_volatile(dest.add(offset), byte) };
    }

    // SAFETY: This writes the exported sequence byte after the packet bytes.
    unsafe {
        write_volatile(
            addr_of_mut!(OBOT_TEXT_API_RESPONSE_PACKET_SEQUENCE),
            packet.sequence,
        );
    }
}

pub fn packet_ptr() -> *const u8 {
    addr_of!(OBOT_BENCHMARK_PACKET).cast::<u8>()
}

pub const fn packet_len() -> usize {
    BENCHMARK_PACKET_LEN
}

pub fn status_packet_ptr() -> *const u8 {
    addr_of!(OBOT_STATUS_PACKET).cast::<u8>()
}

pub const fn status_packet_len() -> usize {
    STATUS_PACKET_LEN
}

pub fn command_packet_ptr() -> *const u8 {
    addr_of!(OBOT_COMMAND_PACKET).cast::<u8>()
}

pub const fn command_packet_len() -> usize {
    COMMAND_PACKET_LEN
}

pub fn driver_command_packet_ptr() -> *const u8 {
    addr_of!(OBOT_DRIVER_COMMAND_PACKET).cast::<u8>()
}

pub const fn driver_command_packet_len() -> usize {
    DRIVER_COMMAND_PACKET_LEN
}

pub fn driver_report_packet_ptr() -> *const u8 {
    addr_of!(OBOT_DRIVER_REPORT_PACKET).cast::<u8>()
}

pub const fn driver_report_packet_len() -> usize {
    DRIVER_REPORT_PACKET_LEN
}

pub fn output_safety_packet_ptr() -> *const u8 {
    addr_of!(OBOT_OUTPUT_SAFETY_PACKET).cast::<u8>()
}

pub const fn output_safety_packet_len() -> usize {
    OUTPUT_SAFETY_PACKET_LEN
}

pub fn bus_voltage_packet_ptr() -> *const u8 {
    addr_of!(OBOT_BUS_VOLTAGE_PACKET).cast::<u8>()
}

pub const fn bus_voltage_packet_len() -> usize {
    BUS_VOLTAGE_PACKET_LEN
}

pub fn text_api_request_packet_ptr() -> *const u8 {
    addr_of!(OBOT_TEXT_API_REQUEST_PACKET).cast::<u8>()
}

pub const fn text_api_request_packet_len() -> usize {
    TEXT_API_REQUEST_PACKET_LEN
}

pub fn text_api_response_packet_ptr() -> *const u8 {
    addr_of!(OBOT_TEXT_API_RESPONSE_PACKET).cast::<u8>()
}

pub const fn text_api_response_packet_len() -> usize {
    TEXT_API_RESPONSE_PACKET_LEN
}
