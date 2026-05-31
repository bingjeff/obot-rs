# obot-rs

Rust firmware workspace for the OBOT STM32G474 motor controller.

This repository is intentionally independent from the adjacent C++ repositories
in `/home/bingjeff/projects/obot`:

- `obot-controller` remains the C++ firmware baseline and hardware bring-up
  reference.
- `motorlib` remains the C++ motor-control library reference.
- `calibration` and `motor_messages` remain separate upstream projects.
- `obot-rs` is the Rust rewrite workspace and has its own Git history.

Do not initialize Git in the parent `obot` directory and do not place this
workspace inside any existing repository. If a tighter relationship is needed
later, add this repo as an explicit submodule or dependency in a separate
change.

## Layout

- `crates/obot-core`: portable `no_std` control state and logic.
- `crates/obot-protocol`: Rust-owned fixed-size host/device packet types.
- `firmware/obot-g474`: STM32G474 firmware integration crate.
- `docs/`: bring-up and repository notes.
- `tools/`: future host-side utilities.

## Checks

```sh
cargo test -p obot-core
cargo test -p obot-protocol
cargo check --workspace
cargo check -p obot-g474 --target thumbv7em-none-eabihf
```

## License

This repository follows `obot-controller` and uses the Unlicense. See `UNLICENSE.txt`.
