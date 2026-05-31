# Repository Layout

`obot-rs` is a sibling repository, not a parent workspace and not a submodule.
This keeps Git operations local to the Rust rewrite and prevents accidental
changes to the existing C++ repositories.

Expected local layout:

```text
/home/bingjeff/projects/obot/
  PLAN.md
  SETUP_README.md
  calibration/
  motor_messages/
  motorlib/
  obot-controller/
  obot-rs/
```

The C++ baseline firmware should still be built from
`obot-controller/obot_g474`. The Rust firmware should be built from `obot-rs`.
Relative paths in documentation are convenience references only; Cargo builds
must not require the sibling repositories.
