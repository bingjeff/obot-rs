#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoopTiming {
    pub cpu_hz: u32,
    pub fast_period_cycles: u32,
    pub main_period_cycles: u32,
}

impl LoopTiming {
    pub const OBOT_G474: Self = Self {
        cpu_hz: 170_000_000,
        fast_period_cycles: 3_400,
        main_period_cycles: 17_000,
    };

    pub const fn new(cpu_hz: u32, fast_period_cycles: u32, main_period_cycles: u32) -> Self {
        Self {
            cpu_hz,
            fast_period_cycles,
            main_period_cycles,
        }
    }

    pub fn from_rates(cpu_hz: u32, fast_hz: u32, main_hz: u32) -> Option<Self> {
        if cpu_hz == 0 || fast_hz == 0 || main_hz == 0 {
            return None;
        }

        if !cpu_hz.is_multiple_of(fast_hz) || !cpu_hz.is_multiple_of(main_hz) {
            return None;
        }

        Some(Self {
            cpu_hz,
            fast_period_cycles: cpu_hz / fast_hz,
            main_period_cycles: cpu_hz / main_hz,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeriodicTask {
    period_cycles: u32,
    next_deadline: u32,
}

impl PeriodicTask {
    pub const fn start(now_cycles: u32, period_cycles: u32) -> Self {
        Self {
            period_cycles,
            next_deadline: now_cycles.wrapping_add(period_cycles),
        }
    }

    pub const fn period_cycles(self) -> u32 {
        self.period_cycles
    }

    pub const fn next_deadline(self) -> u32 {
        self.next_deadline
    }

    pub fn poll(&mut self, now_cycles: u32) -> bool {
        if self.period_cycles == 0 || !deadline_reached(now_cycles, self.next_deadline) {
            return false;
        }

        let elapsed = now_cycles.wrapping_sub(self.next_deadline);
        let periods_elapsed = elapsed / self.period_cycles + 1;
        self.next_deadline = self
            .next_deadline
            .wrapping_add(self.period_cycles.wrapping_mul(periods_elapsed));
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoopScheduler {
    fast: PeriodicTask,
    main: PeriodicTask,
}

impl LoopScheduler {
    pub const fn start(now_cycles: u32, timing: LoopTiming) -> Self {
        Self {
            fast: PeriodicTask::start(now_cycles, timing.fast_period_cycles),
            main: PeriodicTask::start(now_cycles, timing.main_period_cycles),
        }
    }

    pub fn poll(&mut self, now_cycles: u32) -> LoopPoll {
        LoopPoll {
            fast: self.fast.poll(now_cycles),
            main: self.main.poll(now_cycles),
        }
    }

    pub const fn fast_task(self) -> PeriodicTask {
        self.fast
    }

    pub const fn main_task(self) -> PeriodicTask {
        self.main
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoopPoll {
    pub fast: bool,
    pub main: bool,
}

const fn deadline_reached(now_cycles: u32, deadline_cycles: u32) -> bool {
    now_cycles.wrapping_sub(deadline_cycles) as i32 >= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_g474_periods_from_rates() {
        assert_eq!(
            LoopTiming::from_rates(170_000_000, 50_000, 10_000),
            Some(LoopTiming::OBOT_G474)
        );
    }

    #[test]
    fn rejects_zero_rates() {
        assert_eq!(LoopTiming::from_rates(170_000_000, 0, 10_000), None);
        assert_eq!(LoopTiming::from_rates(170_000_000, 50_000, 0), None);
        assert_eq!(LoopTiming::from_rates(0, 50_000, 10_000), None);
    }

    #[test]
    fn rejects_fractional_cycle_periods() {
        assert_eq!(LoopTiming::from_rates(170_000_000, 60_000, 10_000), None);
        assert_eq!(LoopTiming::from_rates(170_000_000, 50_000, 12_000), None);
    }

    #[test]
    fn zero_period_task_never_fires() {
        let mut task = PeriodicTask::start(0, 0);

        assert!(!task.poll(u32::MAX));
        assert_eq!(task.next_deadline(), 0);
    }

    #[test]
    fn periodic_task_fires_on_deadline() {
        let mut task = PeriodicTask::start(0, 3_400);

        assert!(!task.poll(3_399));
        assert_eq!(task.next_deadline(), 3_400);
        assert!(task.poll(3_400));
        assert_eq!(task.next_deadline(), 6_800);
    }

    #[test]
    fn periodic_task_skips_missed_periods() {
        let mut task = PeriodicTask::start(0, 3_400);

        assert!(task.poll(17_000));
        assert_eq!(task.next_deadline(), 20_400);
    }

    #[test]
    fn periodic_task_handles_counter_wrap() {
        let mut task = PeriodicTask::start(u32::MAX - 9, 20);

        assert!(!task.poll(u32::MAX));
        assert!(task.poll(10));
        assert_eq!(task.next_deadline(), 30);
    }

    #[test]
    fn scheduler_reports_fast_and_main_loops() {
        let mut scheduler = LoopScheduler::start(0, LoopTiming::OBOT_G474);

        assert_eq!(
            scheduler.poll(3_400),
            LoopPoll {
                fast: true,
                main: false,
            }
        );
        assert_eq!(
            scheduler.poll(17_000),
            LoopPoll {
                fast: true,
                main: true,
            }
        );
    }
}
