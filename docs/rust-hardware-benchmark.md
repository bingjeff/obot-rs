# Rust Hardware Benchmark Log

## 2026-05-31: Empty Loop Shell, J-Link Debug Readout

Firmware commit: `db7a4ac Configure Rust G474 system clock`

Host helper commit: `2066bb8 Add Rust benchmark debug reader`

Build and flash steps used:

```sh
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
/home/bingjeff/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-objcopy \
  -O binary \
  target/thumbv7em-none-eabihf/release/obot-g474 \
  target/thumbv7em-none-eabihf/release/obot-g474.bin
JLinkExe -CommanderScript /tmp/obot-rs-flash-bin.jlink
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink
```

Release artifact sizes:

```text
132836 target/thumbv7em-none-eabihf/release/obot-g474
  4156 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Flash result:

```text
J-Link: Flash download: Bank 0 @ 0x08000000: 1 range affected (6144 bytes)
J-Link: Flash download: Program & Verify speed: 72 KB/s
O.K.
```

Representative readout after flashing:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 161, 3445, 131, 17007, 74.596, 3399.982, 130.856, 17000.002
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `74.596 cycles = 0.439 us`.
- Main-loop mean execution: `130.856 cycles = 0.770 us`.
- Fast-loop mean period: `3399.982 cycles = 20.000 us`.
- Main-loop mean period: `17000.002 cycles = 100.000 us`.
- Combined 100 us max shell load: `(5 * 161 + 131) / 17000 = 5.51%`.
- Combined 100 us mean shell load: `(5 * 74.596 + 130.856) / 17000 = 2.96%`.

Comparison caveat:

This is only the Rust timing/reporting shell. It does not yet include equivalent ADC/PWM/sensor/control work from the C++ `motor_hall` firmware, so it is not a final performance comparison. It does prove that the Rust timing shell runs on hardware at the intended 20 us and 100 us periods with substantial available headroom before porting motor-control logic.
