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

To verify the release ELF still has no heap allocator dependency:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- verify-no-heap --elf target/thumbv7em-none-eabihf/release/obot-g474
```

To inspect the generated J-Link command script without touching hardware:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- jlink-script --address 0x20000020
```

The decoded output is shaped like `motor_util --run-stats`:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
```

The same benchmark shape can be collected over the Rust firmware's USB text endpoint without J-Link:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- run-stats-usb 100
```

To make the current Rust-vs-C++ performance gate repeatable on hardware, compare the USB benchmark sample against the recorded no-voltage C++ `motor_hall` baseline:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- compare-baseline-usb 100
```

For the accepted unpowered firmware proof, use `accepted-proof-usb`. It reads the board-reported `firmware_version`, checks the driver fail-closed gate, verifies torque/velocity/position commands are observed while outputs stay blocked, collects USB timing stats, and runs the C++ baseline comparison. Pass `--expect-firmware-version` when validating a specific flashed image so the command fails before safety/benchmark checks if the wrong firmware is connected. The current accepted flashed firmware reports `741ffaf`:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- accepted-proof-usb --expect-firmware-version 741ffaf --sequence 200 --samples 100
```

For one-off USB text API reads, use:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-text-api-usb t_exec_fastloop
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-text-api-usb output_allowed
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-text-api-usb driver_configured
```

The USB text endpoint exposes the benchmark fields plus output-safety gates, `bus_voltage_raw`, and the latest DRV8323S configuration report fields needed for powered bring-up checks.

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

To send the same Rust-owned command packet over the USB realtime endpoint and read back the endpoint-2 status response, let the tool discover the OBOT USB device by VID/PID:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-usb --sequence 1 --mode disabled
```

To verify an output-requesting command without leaving it active, `check-command-usb` writes the command, polls realtime status until the command state is observed, reads the safety gates, then sends disabled cleanup by default. On an unpowered/no-VM board, velocity and position commands should be accepted but output-blocked:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- check-command-usb --sequence 1 --mode velocity --velocity 1.25 --expect unpowered-output-blocked
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- check-command-usb --sequence 2 --mode position --position 0.75 --expect unpowered-output-blocked
```

DRV configure/disable commands can use the same USB endpoint. The command returns the immediate status response; read `driver_configured`, `transfer_error_mask`, and related USB text API fields after a short wait for the main-loop DRV result.

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-driver-command-usb --sequence 1 --command configure-enable
```

For a repeatable bring-up check, `check-driver-usb` safely sends a disable first, sends configure-enable, polls the USB text API for the main-loop DRV result, prints the DRV plus safety gates in one row, then sends disable again by default so a powered-ready probe does not leave the DRV enabled. The row includes post-disable fields showing whether cleanup was sent, whether the driver is disabled afterward, and the command-version counters observed after cleanup. Use `--expect unpowered-fail-closed` for the no-VM diagnostic state, or `--expect powered-ready` for the future VM-supplied gate that must pass before considering bridge output enablement. Use `--leave-driver-enabled` only for deliberate follow-on diagnostics that require the DRV to remain enabled after the row is captured.

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- check-driver-usb --sequence 1 --command configure-enable --expect unpowered-fail-closed
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- check-driver-usb --sequence 1 --command configure-enable --expect powered-ready
```

Only run the `powered-ready` probe after the board is prepared for live VM/bus voltage. First run the accepted unpowered proof for the flashed image. A passing powered-ready row must report `check_passed=true`, `driver_configured=true`, `verify_error_mask=0x0000`, `transfer_error_mask=0x0000`, `bus_blocked=false`, `driver_not_enabled=false`, `bridge_prearm_ready=true`, `bridge_prearm_blockers=0x00000000`, and `bridge_outputs_disabled=true`. The default cleanup should also report `post_disable_sent=true` and `post_disable_driver_not_enabled=true`. Do not use `--leave-driver-enabled` for the first live-voltage readiness check.

If more than one OBOT controller is attached, pass the exact `/dev/bus/usb` path reported by `lsusb` with `--dev /dev/bus/usb/<bus>/<dev>`. This uses the Rust packet format on endpoint 2. It is intended for Rust firmware bring-up and is not a drop-in replacement for the original C++ realtime host protocol.

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
