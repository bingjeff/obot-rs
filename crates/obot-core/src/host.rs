#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostCommandWatchdog {
    timeout_ticks: u32,
    ticks_remaining: u32,
    output_command_seen: bool,
    timed_out: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HostCommandWatchdogStatus {
    pub output_allowed: bool,
    pub timed_out: bool,
    pub just_timed_out: bool,
}

impl HostCommandWatchdog {
    pub const fn new(timeout_ticks: u32) -> Self {
        Self {
            timeout_ticks,
            ticks_remaining: 0,
            output_command_seen: false,
            timed_out: false,
        }
    }

    pub fn observe_command(&mut self, command_allows_output: bool) -> HostCommandWatchdogStatus {
        self.output_command_seen = command_allows_output;
        self.ticks_remaining = if command_allows_output {
            self.timeout_ticks
        } else {
            0
        };
        self.timed_out = false;
        self.status(false)
    }

    pub fn tick(&mut self) -> HostCommandWatchdogStatus {
        if !self.output_command_seen {
            self.timed_out = false;
            return self.status(false);
        }

        self.ticks_remaining = self.ticks_remaining.saturating_sub(1);
        let was_timed_out = self.timed_out;
        self.timed_out = self.ticks_remaining == 0;
        self.status(!was_timed_out && self.timed_out)
    }

    const fn status(&self, just_timed_out: bool) -> HostCommandWatchdogStatus {
        HostCommandWatchdogStatus {
            output_allowed: self.output_command_seen && self.ticks_remaining > 0,
            timed_out: self.timed_out,
            just_timed_out,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_or_non_output_commands_do_not_timeout() {
        let mut watchdog = HostCommandWatchdog::new(2);

        assert_eq!(
            watchdog.observe_command(false),
            HostCommandWatchdogStatus::default()
        );
        assert_eq!(watchdog.tick(), HostCommandWatchdogStatus::default());
        assert_eq!(watchdog.tick(), HostCommandWatchdogStatus::default());
    }

    #[test]
    fn output_command_times_out_after_configured_ticks() {
        let mut watchdog = HostCommandWatchdog::new(2);

        assert_eq!(
            watchdog.observe_command(true),
            HostCommandWatchdogStatus {
                output_allowed: true,
                timed_out: false,
                just_timed_out: false,
            }
        );
        assert_eq!(
            watchdog.tick(),
            HostCommandWatchdogStatus {
                output_allowed: true,
                timed_out: false,
                just_timed_out: false,
            }
        );
        assert_eq!(
            watchdog.tick(),
            HostCommandWatchdogStatus {
                output_allowed: false,
                timed_out: true,
                just_timed_out: true,
            }
        );
        assert_eq!(
            watchdog.tick(),
            HostCommandWatchdogStatus {
                output_allowed: false,
                timed_out: true,
                just_timed_out: false,
            }
        );
    }

    #[test]
    fn new_output_command_clears_timeout() {
        let mut watchdog = HostCommandWatchdog::new(1);
        watchdog.observe_command(true);
        assert!(watchdog.tick().timed_out);

        assert_eq!(
            watchdog.observe_command(true),
            HostCommandWatchdogStatus {
                output_allowed: true,
                timed_out: false,
                just_timed_out: false,
            }
        );
    }
}
