# obot-bench-debug

Host helper for decoding the Rust firmware benchmark packet exported at `OBOT_BENCHMARK_PACKET`.

The Rust firmware exports the benchmark packet as `OBOT_BENCHMARK_PACKET` and updates it once per main-loop tick. With a J-Link attached, read and decode it through the built ELF symbol table with:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- run-stats-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

For a one-off read against a known packet address, use:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink --address 0x20000020
```

To inspect the generated J-Link command script without touching hardware:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- jlink-script --address 0x20000020
```

The decoded output is shaped like `motor_util --run-stats`:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
```


For detailed benchmark diagnostics, including sample counts and last-cycle values for each tracked statistic:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink-detail --address 0x20000020
```

The detailed view is useful when a max value is higher than the current steady-state path; it exposes whether the latest sample is near the max or whether the max is a historical tail event.

To inspect why outputs are blocked or allowed, read the output-safety packet through the built ELF symbol table:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-output-safety-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

The output-safety readout includes a `host_timed_out` field so a stale output command is distinguishable from command, bus, driver, and controller gates.

To inspect the raw bus-voltage sample behind the `bus_blocked` gate:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-bus-voltage-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

For bring-up, the combined snapshot reads benchmark, status, driver, output-safety, and bus-voltage packets through the ELF symbol table and reports the 100 us combined load plus remaining cycle headroom:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- snapshot-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

The same snapshot can also be queried through the Rust text API catalog shape that will eventually sit behind USB. For example:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- api-snapshot-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474 mean_fast_loop_cycles
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- api-snapshot-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474 api_name=0
```

After flashing firmware with the text API debug packets, the firmware-owned dispatcher can be exercised through exported SRAM request/response packets:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-text-api-request-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474 --sequence 1 mean_fast_loop_cycles
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-text-api-response-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

To clear a latched controller fault through the debug command packet, use the explicit clear-faults mode:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474 --sequence 1 --mode clear-faults
```

To send the same Rust-owned command packet over the USB realtime endpoint and read back the endpoint-2 status response, use the current `/dev/bus/usb` path reported by `lsusb`:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-usb --dev /dev/bus/usb/001/043 --sequence 1 --mode disabled
```

This uses the Rust packet format on endpoint 2. It is intended for Rust firmware bring-up and is not a drop-in replacement for the original C++ realtime host protocol.


Status and driver report reads also resolve packet addresses from the ELF:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-status-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-driver-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474
```

Driver commands can use the same symbol-resolved path:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-driver-command-jlink --elf target/thumbv7em-none-eabihf/release/obot-g474 --sequence 1 --command disable
```

This is an interim debug readout path. The longer-term compatibility target is still the existing USB/text API used by `motor_util --run-stats`.
