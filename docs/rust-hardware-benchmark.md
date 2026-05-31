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


## 2026-05-31: Current Conversion Surface, J-Link Debug Readout

Firmware commits:

- `7e0a3f6 Add current conversion core`
- `4486180 Enable FPU at startup`

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
133328 target/thumbv7em-none-eabihf/release/obot-g474
  8900 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Representative readout after flashing with FPU enabled:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 354, 3467, 140, 17141, 173.259, 3399.704, 139.355, 16985.418
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `173.259 cycles = 1.019 us`.
- Main-loop mean execution: `139.355 cycles = 0.820 us`.
- Fast-loop mean period: `3399.704 cycles = 19.998 us`.
- Main-loop mean period: `16985.418 cycles = 99.914 us` in the sampled running average.
- Combined 100 us max safe-zero-PWM-plus-hall-plus-current-conversion load: `(5 * 354 + 140) / 17000 = 11.24%`.
- Combined 100 us mean safe-zero-PWM-plus-hall-plus-current-conversion load: `(5 * 173.259 + 139.355) / 17000 = 5.92%`.
- Incremental mean fast-loop cost over raw current ADC readback: `173.259 - 144.001 = 29.258 cycles = 0.172 us`.

Comparison caveat:

This build converts raw ADC3/4/5 current samples using the C++ `motor_hall` gain/bias formula and leaves all HRTIM outputs disabled. FOC d/q transform, current filtering, PI current control, voltage command generation, safety/fault handling, and host API parity remain unimplemented.


## 2026-05-31: Zero-Command FOC Current-Control Surface, J-Link Debug Readout

Firmware commit: `a75f1f2 Add FOC current-control benchmark path`

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
133624 target/thumbv7em-none-eabihf/release/obot-g474
  9744 target/thumbv7em-none-eabihf/release/obot-g474.bin
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
rust_debug, 1007, 3789, 121, 17007, 982, 3399.188, 120.823, 16972.603
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `982 cycles = 5.776 us`.
- Main-loop mean execution: `120.823 cycles = 0.711 us`.
- Fast-loop mean period: `3399.188 cycles = 19.995 us`.
- Main-loop mean period: `16972.603 cycles = 99.839 us` in the sampled running average.
- Combined 100 us max safe-zero-PWM-plus-hall-plus-current-conversion-plus-FOC load: `(5 * 1007 + 121) / 17000 = 30.33%`.
- Combined 100 us mean safe-zero-PWM-plus-hall-plus-current-conversion-plus-FOC load: `(5 * 982 + 120.823) / 17000 = 29.59%`.
- Incremental mean fast-loop cost over current conversion: `982 - 173.259 = 808.741 cycles = 4.757 us`.

Comparison caveat:

This build runs the Rust FOC math with a fixed zero electrical angle, zero current command, measured current conversion, current filtering, PI current control, and voltage command generation. It still discards FOC voltage commands, writes zero PWM, and keeps bridge outputs disabled. It does not yet include hall-derived electrical angle, live command input, PWM voltage application, full safety/fault handling, or host API parity.


## 2026-05-31: Copy-Reduced Zero-Command FOC Surface, J-Link Debug Readout

Firmware commit: `b82df50 Reduce FOC hot-path copies`

Change under test:

- `FocController::step_with_sincos` now takes `&FocCommand` and returns `&FocStatus`, matching the C++ by-reference command/status shape more closely and avoiding one by-value status copy in the measured firmware loop.

Release artifact sizes:

```text
133624 target/thumbv7em-none-eabihf/release/obot-g474
  9728 target/thumbv7em-none-eabihf/release/obot-g474.bin
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
rust_debug, 969, 3772, 121, 17006, 955.19, 3399.231, 120.749, 16974.104
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `955.19 cycles = 5.619 us`.
- Main-loop mean execution: `120.749 cycles = 0.710 us`.
- Combined 100 us max load: `(5 * 969 + 121) / 17000 = 29.21%`.
- Combined 100 us mean load: `(5 * 955.19 + 120.749) / 17000 = 28.80%`.
- Improvement versus the prior FOC benchmark: `982 - 955.19 = 26.81 cycles = 0.158 us` mean fast-loop savings.
- Incremental mean fast-loop cost over current conversion: `955.19 - 173.259 = 781.931 cycles = 4.600 us`.

Comparison caveat:

This is the current best Rust FOC subset result, but it is still not feature-equivalent to C++ `motor_hall`. The hot path still uses a fixed zero electrical angle and does not apply commanded voltage to PWM outputs.


## 2026-05-31: Inlined Zero-Command FOC Surface, J-Link Debug Readout

Firmware commit: `681368e Inline FOC hot path`

Change under test:

- Added `#[inline(always)]` to the FOC hot-path methods in `obot-core` after `llvm-nm -C` showed `FocController::step_with_sincos` remained a standalone cross-crate symbol in the release firmware.
- Rebuilt release firmware and verified the standalone FOC step symbol disappeared from the image.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
llvm-nm -C target/thumbv7em-none-eabihf/release/obot-g474
```

Release artifact sizes:

```text
133496 target/thumbv7em-none-eabihf/release/obot-g474
  9644 target/thumbv7em-none-eabihf/release/obot-g474.bin
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
rust_debug, 836, 3460, 119, 17008, 397.095, 3399.13, 118.798, 16971.1
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `397.095 cycles = 2.336 us`.
- Main-loop mean execution: `118.798 cycles = 0.699 us`.
- Combined 100 us max load: `(5 * 836 + 119) / 17000 = 25.29%`.
- Combined 100 us mean load: `(5 * 397.095 + 118.798) / 17000 = 12.38%`.
- Improvement versus the copy-reduced FOC benchmark: `955.19 - 397.095 = 558.095 cycles = 3.283 us` mean fast-loop savings.
- Incremental mean fast-loop cost over current conversion: `397.095 - 173.259 = 223.836 cycles = 1.317 us`.

Comparison caveat:

This is the current best Rust FOC subset result. Its mean fast-loop cost is now below the recorded C++ full fast-loop mean (`397.095` cycles Rust subset versus `708.965` cycles C++ baseline), but it remains not feature-equivalent because the Rust path still uses a fixed zero electrical angle and does not apply voltage commands to PWM outputs. The observed max sample (`836` cycles) is still higher than the C++ max fast-loop sample (`710` cycles), so max-latency variance remains worth tracking as functionality is added.


## 2026-05-31: Hall-Derived FOC Angle Surface, J-Link Debug Readout

Firmware commits:

- `c7ed6aa Add hall-derived FOC angle`
- `f5cf5f5 Use hall sector for FOC sincos`
- `5f2fd92 Inline hall sampling path`

Change under test:

- Rust FOC now uses the live hall count to derive the electrical angle for the `motor_hall` parameter set.
- `phase_mode = 1` is represented as phase sign `-1`, matching the C++ `set_phase_mode()` behavior.
- Runtime trigonometry is avoided for hall input: the current 1..6 hall sector maps directly to a six-entry electrical sin/cos table.
- The firmware still writes zero PWM and keeps bridge outputs disabled.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
```

Release artifact sizes:

```text
133560 target/thumbv7em-none-eabihf/release/obot-g474
  9700 target/thumbv7em-none-eabihf/release/obot-g474.bin
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
rust_debug, 919, 3442, 127, 17372, 433.651, 3399.125, 126.391, 16969.975
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `433.651 cycles = 2.551 us`.
- Main-loop mean execution: `126.391 cycles = 0.744 us`.
- Combined 100 us max load: `(5 * 919 + 127) / 17000 = 27.78%`.
- Combined 100 us mean load: `(5 * 433.651 + 126.391) / 17000 = 13.50%`.
- Incremental mean fast-loop cost over the fixed-angle inlined FOC result: `433.651 - 397.095 = 36.556 cycles = 0.215 us`.
- Incremental mean fast-loop cost over current conversion: `433.651 - 173.259 = 260.392 cycles = 1.532 us`.

Comparison caveat:

This is closer to the C++ `motor_hall` control path because it uses live hall-derived electrical angle, current conversion, FOC transforms, current filtering, PI current control, and voltage command generation. It is still not feature-equivalent: commanded voltages are not applied to PWM, outputs remain disabled, live host commands are absent, and full safety/fault behavior is not implemented. Mean fast-loop cost is below the C++ full fast-loop mean (`433.651` cycles Rust subset versus `708.965` cycles C++ baseline), but the max sample remains higher (`919` cycles Rust subset versus `710` cycles C++ baseline).
