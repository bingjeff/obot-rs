use core::ptr::{addr_of, addr_of_mut, write_volatile};

use obot_protocol::{BENCHMARK_PACKET_LEN, BenchmarkPacket};

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BENCHMARK_PACKET: [u8; BENCHMARK_PACKET_LEN] = [0; BENCHMARK_PACKET_LEN];

#[unsafe(no_mangle)]
#[used]
pub static mut OBOT_BENCHMARK_PACKET_SEQUENCE: u8 = 0;

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

pub fn packet_ptr() -> *const u8 {
    addr_of!(OBOT_BENCHMARK_PACKET).cast::<u8>()
}

pub const fn packet_len() -> usize {
    BENCHMARK_PACKET_LEN
}
