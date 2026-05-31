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

To clear a latched controller fault through the debug command packet, use the explicit clear-faults mode:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-jlink --packet-address 0x20000071 --sequence-address 0x2000009e --sequence 1 --mode clear-faults
```

This is an interim debug readout path. The longer-term compatibility target is still the existing USB/text API used by `motor_util --run-stats`.
