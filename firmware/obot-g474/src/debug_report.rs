use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use obot_protocol::{
    BENCHMARK_PACKET_LEN, BenchmarkPacket, COMMAND_PACKET_LEN, CommandPacket,
    DRIVER_COMMAND_PACKET_LEN, DRIVER_REPORT_PACKET_LEN, DriverCommandPacket, DriverReportPacket,
    STATUS_PACKET_LEN, StatusPacket,
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
            *last_sequence = sequence;
            Some(packet)
        }
        Ok(_) | Err(_) => {
            *last_sequence = sequence;
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
            *last_sequence = sequence;
            Some(packet)
        }
        Ok(_) | Err(_) => {
            *last_sequence = sequence;
            None
        }
    }
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
