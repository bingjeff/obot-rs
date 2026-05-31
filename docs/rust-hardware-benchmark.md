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



## 2026-05-31: Disabled PWM Compare Computation Surface, J-Link Debug Readout

Firmware commits:

- `37372a9 Add disabled PWM compare path`
- `bf3de69 Benchmark PWM compare computation only`
- `40ce155 Precompute PWM voltage scale`
- `03bdd5c Precompute PWM clamp bounds`
- `f691b88 Return FOC status by value`
- `62fd74e Restore faster FOC benchmark retention`

Change under test:

- Added the next output-path surface: calculate clamped HRTIM compare values from FOC phase-voltage commands using the C++ `motor_hall` PWM shape.
- Kept bridge outputs disabled. The current measured firmware writes zero PWM each fast loop, computes candidate phase compares from FOC output, and does not yet write those candidate compares to HRTIM in the benchmarked path.
- A register-writing disabled path was also implemented for later use, but measured as too expensive for the current increment while outputs are disabled; compute-only is the current benchmark surface.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
```

Optimization trail:

```text
Disabled compare writes:        rust_debug, 1150, 3901, 122, 17008, 1149.361, 3399.019, 121.81, 16970.074
Compute-only before cleanup:    rust_debug, 1138, 3876, 110, 17007, 1137.289, 3399.049, 109.817, 16970.967
Precomputed voltage scale:      rust_debug, 1122, 3622, 111, 17007, 1120.863, 3398.287, 110.743, 16949.635
Precomputed clamp bounds:       rust_debug, 1093, 3561, 112, 17458, 1092.383, 3397.746, 110.708, 16934.303
FOC status returned by value:   rust_debug, 977, 3485, 111, 17009, 976.961, 3399.079, 110.759, 16963.032
Reference retention regression: rust_debug, 1093, 3561, 112, 17009, 1092.928, 3398.268, 111.677, 16948.546
Restored current best:          rust_debug, 977, 3485, 111, 17009, 976.951, 3398.908, 110.718, 16956.188
```

Release artifact sizes for current best:

```text
133620 target/thumbv7em-none-eabihf/release/obot-g474
 10240 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Flash result:

```text
J-Link: Flash download: Bank 0 @ 0x08000000: 1 range affected (10240 bytes)
J-Link: Flash download: Program & Verify speed: 77 KB/s
O.K.
```

Representative readout after flashing current best:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 977, 3485, 111, 17009, 976.951, 3398.908, 110.718, 16956.188
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `976.951 cycles = 5.747 us`.
- Main-loop mean execution: `110.718 cycles = 0.651 us`.
- Combined 100 us max load: `(5 * 977 + 111) / 17000 = 29.39%`.
- Combined 100 us mean load: `(5 * 976.951 + 110.718) / 17000 = 29.39%`.
- Incremental mean fast-loop cost over hall-derived FOC: `976.951 - 433.651 = 543.300 cycles = 3.196 us`.

Current comparison:

This Rust path now includes live hall-derived electrical angle, current conversion, FOC transforms, current filtering, PI current control, voltage command generation, and candidate PWM compare computation. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not yet applied to HRTIM in the benchmarked path, bridge outputs remain disabled, live host commands are absent, and full bus-voltage/fault gating is not implemented.

The current Rust combined mean load remains below the recorded C++ no-voltage `motor_hall` baseline (`29.39%` versus `41.77%`), but Rust fast-loop execution is now slower than the C++ fast-loop mean (`976.951` cycles versus `708.965` cycles), and the Rust max sample remains higher (`977` cycles versus `710` cycles). The next performance work should focus on the PWM compare conversion/code shape before enabling real output writes.



## 2026-05-31: Integer-Clamped PWM Compare Conversion, J-Link Debug Readout

Firmware commit: `fd8e0c9 Clamp PWM compares as integers`

Change under test:

- Changed PWM compare conversion from float-domain clamp plus float-to-u32 conversion to float scaling, float-to-i32 conversion, and integer-domain clamp.
- This removes the VFP compare/`vmrs` sequence from the PWM compare bound checks while keeping final compare values clamped to the same HRTIM register range.
- Bridge outputs remain disabled; the measured path still writes zero PWM and computes candidate compare values only.

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
135788 target/thumbv7em-none-eabihf/release/obot-g474
 12372 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

Flash result:

```text
J-Link: Flash download: Bank 0 @ 0x08000000: 1 range affected (14336 bytes)
J-Link: Flash download: Program & Verify speed: 79 KB/s
O.K.
```

Representative readouts after flashing:

```text
rust_debug, 935, 3423, 136, 17006, 425.384, 3399.006, 135.695, 16956.187
rust_debug, 935, 3433, 136, 17009, 425.045, 3399.443, 135.823, 16974.811
rust_debug, 935, 3433, 136, 17009, 424.918, 3399.604, 135.873, 16982.034
```

Interpretation at 170 MHz using the best repeat readout:

- Fast-loop mean execution: `424.918 cycles = 2.499 us`.
- Main-loop mean execution: `135.873 cycles = 0.799 us`.
- Combined 100 us max load: `(5 * 935 + 136) / 17000 = 28.30%`.
- Combined 100 us mean load: `(5 * 424.918 + 135.873) / 17000 = 13.30%`.
- Mean fast-loop improvement over previous PWM compare best: `976.951 - 424.918 = 552.033 cycles = 3.247 us`.
- Incremental mean fast-loop cost over hall-derived FOC without PWM compares: `424.918 - 433.651 = -8.733 cycles`; this is within benchmark/code-shape noise and shows the candidate compare computation is no longer the dominant cost.

Current comparison:

This is the current best Rust measured path with live hall-derived electrical angle, current conversion, FOC transforms, current filtering, PI current control, voltage command generation, and candidate PWM compare computation. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not yet applied to HRTIM in the benchmarked path, bridge outputs remain disabled, live host commands are absent, and full bus-voltage/fault gating is not implemented.

The current Rust fast-loop mean is below the recorded C++ no-voltage `motor_hall` fast-loop mean (`424.918` cycles Rust versus `708.965` cycles C++), and the Rust combined mean load remains below C++ (`13.30%` versus `41.77%`). The Rust max sample is still higher than the C++ max fast-loop sample (`935` cycles Rust versus `710` cycles C++), so max-latency variance remains a tracking item as output gating and host command handling are added.



## 2026-05-31: Bus Voltage Monitor With Cached Fast-Loop Gate, J-Link Debug Readout

Firmware commits:

- `8b1c546 Add bus voltage output gate`
- `5c96bb1 Use raw bus voltage gate in fast loop`
- `aaafbfb Measure bus gate without command masking`
- `efb841e Match motor_hall bus voltage limits`
- `8432105 Split bus gate benchmark retention`
- `80cd0eb Monitor bus voltage from main loop`
- `cbebe67 Keep FOC benchmark retention separate`
- `660a9b9 Isolate bus voltage monitor codegen`
- `143021b Gate fast loop from cached bus voltage`

Change under test:

- Added ADC1/OPAMP1 setup for the active `motor_hall` bus-voltage channel.
- Added Rust bus-voltage calibration and output limits matching the active C++ config: `vbus_gain = 1.0 / 4096 * (215 + 13.7) / 13.7`, default `vbus_min = 8 V`, default `vbus_max = 60 V`.
- Measured the direct ADC1 fast-loop read path and found it raised mean fast-loop execution to roughly `975-1030` cycles.
- Current staged design updates a cached bus-voltage raw sample in the 10 kHz main loop and evaluates the output gate from that cached sample in the 50 kHz fast loop.
- Bridge outputs remain disabled; the measured fast loop still writes zero PWM and computes candidate compares only.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
```

Diagnostic readouts during this increment:

```text
Fast-loop ADC1 read plus raw gate:       rust_debug, 1001, 3488, 143, 17623, 1000.371, 3397.197, 141.784, 16902.697
Fast-loop ADC1 read, no command mask:    rust_debug, 975, 3468, 140, 17003, 974.989, 3397.483, 139.625, 16912.162
ADC1 initialized, no fast-loop DR read:  rust_debug, 941, 3439, 142, 17001, 440.445, 3397.699, 141.650, 16915.753
Cached fast-loop gate current best:      rust_debug, 927, 3569, 269, 17008, 436.896, 3397.505, 268.724, 16910.021
```

Release artifact sizes for current best:

```text
135876 target/thumbv7em-none-eabihf/release/obot-g474
 13856 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

`arm-none-eabi-size` for current best:

```text
   text   data    bss    dec    hex filename
  13856      0     84  13940   3674 target/thumbv7em-none-eabihf/release/obot-g474
```

Representative readout after flashing current best:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 927, 3569, 269, 17008, 436.896, 3397.505, 268.724, 16910.021
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `436.896 cycles = 2.570 us`.
- Main-loop mean execution: `268.724 cycles = 1.581 us`.
- Combined 100 us max load: `(5 * 927 + 269) / 17000 = 28.85%`.
- Combined 100 us mean load: `(5 * 436.896 + 268.724) / 17000 = 14.43%`.
- Incremental mean fast-loop cost over integer-clamped PWM compare: `436.896 - 424.918 = 11.978 cycles = 0.070 us`.
- Incremental mean main-loop cost over integer-clamped PWM compare: `268.724 - 135.873 = 132.851 cycles = 0.781 us`.

Current comparison:

This Rust path now includes live hall-derived electrical angle, current conversion, FOC transforms, current filtering, PI current control, voltage command generation, candidate PWM compare computation, ADC1 bus-voltage monitoring, and a cached fast-loop output-gate decision. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not yet applied to HRTIM, bridge outputs remain disabled, live host commands are absent, driver fault handling is absent, and full safe-mode/fault behavior is not implemented.

The current Rust fast-loop mean remains below the recorded C++ no-voltage `motor_hall` fast-loop mean (`436.896` cycles Rust versus `708.965` cycles C++), and Rust combined mean load remains below C++ (`14.43%` versus `41.77%`). Rust combined max load is also below C++ (`28.85%` versus `58.79%`), but the Rust max fast-loop sample remains higher than the C++ fast-loop max (`927` cycles Rust versus `710` cycles C++), so max-latency variance remains a tracking item.



## 2026-05-31: Output Safety State And Driver Fault Pins, J-Link Debug Readout

Firmware commits:

- `c8b35b1 Add output safety and driver pins`
- `32981a1 Cache output safety for fast loop`
- `687d6d3 Keep output safety latch off stack`

Change under test:

- Added a core `OutputSafety` state machine with latching driver-fault behavior.
- Added G474 motor driver pins for active `motor_hall` hardware: PC13 driver enable output initialized disabled, and PC14 active-low driver fault input with pull-up.
- The firmware samples driver pins and updates the output safety latch in the 10 kHz main loop, then uses a cached output-allowed decision in the 50 kHz fast loop.
- `command_allows_output` remains hard-coded false until the host command path exists, so outputs remain disabled and PWM continues to write zero voltage.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
```

Diagnostic readouts during this increment:

```text
Driver pins plus safety update in fast loop: rust_debug, 1019, 3733, 269, 17001, 1017.772, 3395.832, 268.515, 16868.855
Cached safety with latch on stack:           rust_debug, 918, 3705, 419, 17852, 899.523, 3397.293, 416.327, 16910.282
Cached safety with static latch:             rust_debug, 924, 3711, 408, 17009, 433.885, 3397.309, 407.614, 16907.536
```

Release artifact sizes for current best:

```text
136064 target/thumbv7em-none-eabihf/release/obot-g474
 14060 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

`arm-none-eabi-size` for current best:

```text
   text   data    bss    dec    hex filename
  14060      0     84  14144   3740 target/thumbv7em-none-eabihf/release/obot-g474
```

Representative readout after flashing current best:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 924, 3711, 408, 17009, 433.885, 3397.309, 407.614, 16907.536
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `433.885 cycles = 2.552 us`.
- Main-loop mean execution: `407.614 cycles = 2.398 us`.
- Combined 100 us max load: `(5 * 924 + 408) / 17000 = 29.58%`.
- Combined 100 us mean load: `(5 * 433.885 + 407.614) / 17000 = 15.16%`.
- Incremental mean fast-loop cost versus cached bus gate: `433.885 - 436.896 = -3.011 cycles`, within benchmark/code-shape noise.
- Incremental mean main-loop cost versus cached bus gate: `407.614 - 268.724 = 138.890 cycles = 0.817 us`.

Current comparison:

This Rust path now includes live hall-derived electrical angle, current conversion, FOC transforms, current filtering, PI current control, voltage command generation, candidate PWM compare computation, ADC1 bus-voltage monitoring, driver fault/enable pin sampling, latching output safety state, and a cached fast-loop output gate. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not yet applied to HRTIM, bridge outputs remain disabled, live host commands are absent, DRV8323S SPI register setup/status readback is absent, and full safe-mode/fault behavior is not implemented.

The current Rust fast-loop mean remains below the recorded C++ no-voltage `motor_hall` fast-loop mean (`433.885` cycles Rust versus `708.965` cycles C++), and Rust combined mean load remains below C++ (`15.16%` versus `41.77%`). Rust combined max load is also below C++ (`29.58%` versus `58.79%`), but the Rust max fast-loop sample remains higher than the C++ fast-loop max (`924` cycles Rust versus `710` cycles C++), so max-latency variance remains a tracking item.


## 2026-05-31: Host Debug Command/Status Channel, J-Link Debug Readout

Firmware commits:

- `fa04ef0 Add debug command status channel`
- `c520ef0 Keep controller state out of fast loop`
- `92cd616 Isolate debug command state from fast loop`

Host helper change under test:

- `write-command-jlink` now emits explicit J-Link `w1` byte writes for the command packet followed by the command sequence byte.
- This replaced the temporary `loadbin` SRAM writer after live testing showed `loadbin` did not reliably update the running target's command packet, while `w1` did.

Verified commands:

```sh
cargo test --manifest-path tools/obot-bench-debug/Cargo.toml
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-jlink --packet-address 0x20000071 --sequence-address 0x2000008e --sequence 8 --mode torque --torque 1.5
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-status-jlink --address 0x2000007f
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink --address 0x20000020
```

Current exported packet addresses for firmware commit `92cd616`:

```text
OBOT_BENCHMARK_PACKET          0x20000020
OBOT_COMMAND_PACKET            0x20000071
OBOT_STATUS_PACKET             0x2000007f
OBOT_COMMAND_PACKET_SEQUENCE   0x2000008e
```

Live command/status result:

```text
name, sequence, fault, torque_nm, velocity_rad_s, position_rad
rust_debug, 94, none, 1.5, 0, 0
```

Representative benchmark readout after the command/status channel:

```text
name, max_fast_loop_cycles, max_fast_loop_period, max_main_loop_cycles, max_main_loop_period, mean_fast_loop_cycles, mean_fast_loop_period, mean_main_loop_cycles, mean_main_loop_period
rust_debug, 904, 4559, 1336, 17010, 424.798, 3400.004, 873.016, 16999.677
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `424.798 cycles = 2.499 us`.
- Main-loop mean execution: `873.016 cycles = 5.135 us`.
- Combined 100 us max load: `(5 * 904 + 1336) / 17000 = 34.45%`.
- Combined 100 us mean load: `(5 * 424.798 + 873.016) / 17000 = 17.63%`.

Current comparison:

The command/status channel keeps the extra work in the 10 kHz main-loop path. The Rust fast-loop mean remains below the recorded C++ no-voltage `motor_hall` fast-loop mean (`424.798` cycles Rust versus `708.965` cycles C++), and the combined mean remains below C++ (`17.63%` versus `41.77%`). The firmware is still not feature-equivalent: HRTIM compares are not applied, bridge outputs remain disabled, DRV8323S SPI setup/status is absent, USB/motor_util API compatibility is absent, and full safe-mode/fault behavior is not implemented.


## 2026-05-31: DRV8323S SPI Status Surface, J-Link Debug Readout

Change under test:

- Added `firmware/obot-g474/src/drv8323s.rs` for the active `motor_hall` DRV8323S SPI pins: PA4 NSS, PA5 SCK, PA6 MISO with pull-up, and PA7 MOSI.
- Configures SPI1 for the same broad transaction shape as the C++ driver: master, 16-bit TI mode, baud prescaler `/64`.
- Adds bounded transfer timeouts and a startup-only status probe for DRV8323S registers 0 and 1.
- Keeps PC13 driver enable low and HRTIM bridge outputs disabled.
- The DRV startup path is marked cold/noinline so the one-shot SPI path does not perturb the measured 50 kHz hot loop.

Verified commands:

```sh
cargo test --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
JLinkExe -CommanderScript /tmp/obot-rs-flash-bin.jlink
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink --address 0x20000020
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-command-jlink --packet-address 0x20000071 --sequence-address 0x2000008e --sequence 9 --mode torque --torque 1.25
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-status-jlink --address 0x2000007f
```

Release artifact sizes:

```text
137052 target/thumbv7em-none-eabihf/release/obot-g474
 15296 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

`arm-none-eabi-size`:

```text
   text   data    bss    dec    hex filename
  15232     32    116  15380   3c14 target/thumbv7em-none-eabihf/release/obot-g474
```

Representative benchmark readouts after flashing:

```text
rust_debug, 903, 3725, 865, 17009, 410.579, 3399.701, 864.892, 16985.346
rust_debug, 903, 4118, 1334, 17010, 410.289, 3399.879, 865.585, 16993.859
```

Command/status verification after this firmware change:

```text
name, sequence, fault, torque_nm, velocity_rad_s, position_rad
rust_debug, 63, none, 1.25, 0, 0
```

Interpretation at 170 MHz using the final readout:

- Fast-loop mean execution: `410.289 cycles = 2.413 us`.
- Main-loop mean execution: `865.585 cycles = 5.091 us`.
- Combined 100 us max load: `(5 * 903 + 1334) / 17000 = 34.41%`.
- Combined 100 us mean load: `(5 * 410.289 + 865.585) / 17000 = 17.16%`.

Rejected variant:

A periodic DRV status read inside the measured 10 kHz main loop booted and ran, but raised main-loop max to `5579` cycles and combined max to about `59.08%`, slightly above the C++ combined max baseline. Keep periodic DRV polling out of the benchmarked main loop until it is designed as an explicit slower maintenance task or host-triggered operation.

Current comparison:

The Rust path now includes the earlier motor-control subset, output safety gates, J-Link command/status, and a cold startup DRV8323S SPI status surface. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not applied to HRTIM, bridge outputs remain disabled, DRV8323S register programming is not implemented, USB/motor_util API compatibility is absent, and full safe-mode/fault behavior is not implemented.


## 2026-05-31: DRV8323S Register Programming With Readback Report, J-Link Debug Readout

Change under test:

- Added the active C++ `motor_hall` DRV8323S register values to Rust: `0x1000`, `0x1BFF`, `0x237F`, `0x2800`, and `0x32C0`.
- Added a startup programming pass that writes each register, reads the same address back, and compares the low 11 data bits.
- Added transfer and verification masks plus a 14-byte driver report packet readable through J-Link.
- Kept PC13 driver enable low and HRTIM outputs disabled.

Verified commands:

```sh
cargo test --workspace
cargo test --manifest-path tools/obot-bench-debug/Cargo.toml
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
JLinkExe -CommanderScript /tmp/obot-rs-flash-bin.jlink
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-driver-jlink --address 0x2000007f
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink --address 0x20000020
```

Release artifact sizes:

```text
137264 target/thumbv7em-none-eabihf/release/obot-g474
 16148 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

`arm-none-eabi-size`:

```text
   text   data    bss    dec    hex filename
  16116     32    132  16280   3f98 target/thumbv7em-none-eabihf/release/obot-g474
```

Exported debug packet addresses for this build:

```text
OBOT_BENCHMARK_PACKET          0x20000020
OBOT_COMMAND_PACKET            0x20000071
OBOT_DRIVER_REPORT_PACKET      0x2000007f
OBOT_STATUS_PACKET             0x2000008d
OBOT_COMMAND_PACKET_SEQUENCE   0x2000009c
```

Driver report readout:

```text
name, sequence, configured, verify_error_mask, transfer_error_mask, status_before, status_after
rust_debug, 0, false, 0x001F, 0x0000, 0xFFFFFFFF, 0xFFFFFFFF
```

Interpretation:

- SPI transfers completed: `transfer_error_mask = 0x0000`.
- All five configured register readbacks mismatched: `verify_error_mask = 0x001F`.
- Status before/after both read as `0xFFFFFFFF`.
- This is consistent with the current safety staging because PC13 remains disabled. The programming/readback machinery is present and observable; successful DRV configuration is deferred to the explicit driver-enable stage.

Representative benchmark after flashing and command verification:

```text
rust_debug, 903, 4213, 1335, 17008, 410.408, 3399.954, 866.489, 16983.596
```

Command/status verification at shifted addresses:

```text
name, sequence, fault, torque_nm, velocity_rad_s, position_rad
rust_debug, 255, none, 1.25, 0, 0
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `410.408 cycles = 2.414 us`.
- Main-loop mean execution: `866.489 cycles = 5.097 us`.
- Combined 100 us max load: `(5 * 903 + 1335) / 17000 = 34.41%`.
- Combined 100 us mean load: `(5 * 410.408 + 866.489) / 17000 = 17.17%`.

Current comparison:

The Rust path now includes the earlier motor-control subset, output safety gates, J-Link command/status, and DRV8323S register programming/readback reporting. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not applied to HRTIM, bridge outputs remain disabled, successful DRV8323S configuration requires an explicit driver-enable stage, USB/motor_util API compatibility is absent, and full safe-mode/fault behavior is not implemented.


## 2026-05-31: Host-Triggered DRV8323S Configure Enable Command, J-Link Debug Readout

Change under test:

- Added a separate driver command packet and `write-driver-command-jlink` helper.
- Firmware now waits for a host `configure-enable` command before asserting PC13 and running DRV8323S register programming/readback.
- On failed verification, firmware disables PC13 again.
- HRTIM outputs remain disabled and the fast loop still writes zero voltage.

Verified commands:

```sh
cargo test --workspace
cargo test --manifest-path tools/obot-bench-debug/Cargo.toml
cargo check -p obot-g474 --target thumbv7em-none-eabihf
cargo build -p obot-g474 --release --target thumbv7em-none-eabihf
cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --manifest-path tools/obot-bench-debug/Cargo.toml -- -D warnings
JLinkExe -CommanderScript /tmp/obot-rs-flash-bin.jlink
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- write-driver-command-jlink --packet-address 0x2000007f --sequence-address 0x2000009f --sequence 1 --command configure-enable
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-driver-jlink --address 0x20000081
cargo run --manifest-path tools/obot-bench-debug/Cargo.toml -- read-jlink --address 0x20000020
```

Release artifact sizes:

```text
137752 target/thumbv7em-none-eabihf/release/obot-g474
 16864 target/thumbv7em-none-eabihf/release/obot-g474.bin
```

`arm-none-eabi-size`:

```text
   text   data    bss    dec    hex filename
  16832     32    132  16996   4264 target/thumbv7em-none-eabihf/release/obot-g474
```

Exported packet addresses:

```text
OBOT_BENCHMARK_PACKET                 0x20000020
OBOT_COMMAND_PACKET                   0x20000071
OBOT_DRIVER_COMMAND_PACKET            0x2000007f
OBOT_DRIVER_REPORT_PACKET             0x20000081
OBOT_STATUS_PACKET                    0x2000008f
OBOT_COMMAND_PACKET_SEQUENCE          0x2000009e
OBOT_DRIVER_COMMAND_PACKET_SEQUENCE   0x2000009f
```

Initial driver report after flash:

```text
rust_debug, 0, false, 0x0000, 0x0000, 0x00000000, 0x00000000
```

Driver report after `configure-enable`:

```text
name, sequence, configured, verify_error_mask, transfer_error_mask, status_before, status_after
rust_debug, 0, false, 0x001F, 0x0000, 0xFFFFFFFF, 0xFFFFFFFF
```

GPIOC IDR after failed configure:

```text
0x48000810 = 0x00004000
```

Interpretation:

- The command path executed and generated a real report.
- SPI transfers completed, but all five readbacks mismatched and DRV status read as all ones.
- PC13 was low after failure and PC14 fault input was high, so failed configure did not leave the driver-enable pin asserted.

Representative steady-state benchmark after configure command and benchmark reset:

```text
rust_debug, 892, 4578, 1551, 17008, 409.872, 3400.051, 1071.433, 16976.8
```

Motor command/status verification:

```text
name, sequence, fault, torque_nm, velocity_rad_s, position_rad
rust_debug, 39, none, 1.25, 0, 0
```

Interpretation at 170 MHz:

- Fast-loop mean execution: `409.872 cycles = 2.411 us`.
- Main-loop mean execution: `1071.433 cycles = 6.302 us`.
- Combined 100 us max load: `(5 * 892 + 1551) / 17000 = 35.36%`.
- Combined 100 us mean load: `(5 * 409.872 + 1071.433) / 17000 = 18.89%`.

Current comparison:

The Rust path now includes the earlier motor-control subset, output safety gates, J-Link motor command/status, host-triggered driver command handling, and DRV8323S register programming/readback reporting. It is still not feature-equivalent to C++ `motor_hall`: candidate compares are not applied to HRTIM, bridge outputs remain disabled, successful DRV8323S configuration is not yet achieved on this attached hardware state, USB/motor_util API compatibility is absent, and full safe-mode/fault behavior is not implemented.

