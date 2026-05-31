# obot-bench-debug

Host helper for decoding the Rust firmware benchmark packet exported at `OBOT_BENCHMARK_PACKET`.

The Rust firmware currently exports the packet at `0x20000000` and updates it once per main-loop tick. With a J-Link attached, read and decode it with:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink
```

To inspect the generated J-Link command script without touching hardware:

```sh
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- jlink-script
```

The decoded output is shaped like `motor_util --run-stats`:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
```

This is an interim debug readout path. The longer-term compatibility target is still the existing USB/text API used by `motor_util --run-stats`.
