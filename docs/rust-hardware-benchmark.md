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


## 2026-05-31: Safe Zero-PWM HRTIM Surface, J-Link Debug Readout

Firmware commit: `cfdc9ff Add safe zero PWM surface`

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
132988 target/thumbv7em-none-eabihf/release/obot-g474
  4500 target/thumbv7em-none-eabihf/release/obot-g474.bin
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
rust_debug, 194, 3449, 131, 17007, 96.589, 3399.98, 130.872, 17000.004
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `96.589 cycles = 0.568 us`.
- Main-loop mean execution: `130.872 cycles = 0.770 us`.
- Fast-loop mean period: `3399.98 cycles = 20.000 us`.
- Main-loop mean period: `17000.004 cycles = 100.000 us`.
- Combined 100 us max safe-zero-PWM load: `(5 * 194 + 131) / 17000 = 6.48%`.
- Combined 100 us mean safe-zero-PWM load: `(5 * 96.589 + 130.872) / 17000 = 3.61%`.
- Incremental mean fast-loop cost over the empty shell: `96.589 - 74.596 = 21.993 cycles = 0.129 us`.

Comparison caveat:

This build configures the HRTIM PWM timing surface and writes zero-voltage compare values every fast loop, but it keeps all HRTIM outputs disabled with `ODISR = 0x0FFF`. It still does not include ADC sampling, hall sensor decoding, current control, voltage control, safety/fault handling, or host API parity.


## 2026-05-31: Hall GPIO Input Surface, J-Link Debug Readout

Firmware commit: `5cea313 Add hall GPIO readback`

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
133188 target/thumbv7em-none-eabihf/release/obot-g474
  4660 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Representative readout after flashing:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 237, 3465, 122, 17009, 118.577, 3399.991, 121.954, 17000.003
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `118.577 cycles = 0.697 us`.
- Main-loop mean execution: `121.954 cycles = 0.717 us`.
- Fast-loop mean period: `3399.991 cycles = 20.000 us`.
- Main-loop mean period: `17000.003 cycles = 100.000 us`.
- Combined 100 us max safe-zero-PWM-plus-hall load: `(5 * 237 + 122) / 17000 = 7.69%`.
- Combined 100 us mean safe-zero-PWM-plus-hall load: `(5 * 118.577 + 121.954) / 17000 = 4.20%`.
- Incremental mean fast-loop cost over safe zero-PWM only: `118.577 - 96.589 = 21.988 cycles = 0.129 us`.

Comparison caveat:

This build configures and reads PA0/PA1/PA2 hall inputs and runs the same hall lookup/wrap logic as the C++ `HallEncoder`. It keeps all HRTIM outputs disabled and still does not include ADC sampling, current control, voltage control, safety/fault handling, or host API parity.


## 2026-05-31: Current ADC Injected Sample Surface, J-Link Debug Readout

Firmware commit: `0353082 Add current ADC readback`

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
133292 target/thumbv7em-none-eabihf/release/obot-g474
  8760 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Flash result:

```text
J-Link: Flash download: Bank 0 @ 0x08000000: 1 range affected (10240 bytes)
J-Link: Flash download: Program & Verify speed: 77 KB/s
O.K.
```

Representative readout after flashing:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 292, 3464, 131, 17006, 144.001, 3399.798, 130.929, 16989.648
```

Direct ADC injected data register readback from the running firmware:

```text
ADC3 JDR1 @ 0x50000480 = 0x0331
ADC4 JDR1 @ 0x50000580 = 0x069F
ADC5 JDR1 @ 0x50000680 = 0x05FC
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `144.001 cycles = 0.847 us`.
- Main-loop mean execution: `130.929 cycles = 0.770 us`.
- Fast-loop mean period: `3399.798 cycles = 19.999 us`.
- Main-loop mean period: `16989.648 cycles = 99.939 us` in the sampled running average.
- Combined 100 us max safe-zero-PWM-plus-hall-plus-current-ADC load: `(5 * 292 + 131) / 17000 = 9.36%`.
- Combined 100 us mean safe-zero-PWM-plus-hall-plus-current-ADC load: `(5 * 144.001 + 130.929) / 17000 = 5.01%`.
- Incremental mean fast-loop cost over hall GPIO readback: `144.001 - 118.577 = 25.424 cycles = 0.150 us`.

Comparison caveat:

This build configures ADC3/4/5 injected current channels, starts HRTIM-triggered injected conversions, and reads the resulting injected data registers in the fast loop. It keeps all HRTIM outputs disabled and still does not include ADC1/ADC2 housekeeping, bus voltage, temperature/vref scaling, current-control math, voltage control, safety/fault handling, or host API parity.
