#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CycleStats {
    samples: u32,
    last_cycles: u32,
    max_cycles: u32,
    total_cycles: u64,
}

impl CycleStats {
    pub const fn new() -> Self {
        Self {
            samples: 0,
            last_cycles: 0,
            max_cycles: 0,
            total_cycles: 0,
        }
    }

    pub fn record(&mut self, cycles: u32) {
        self.samples = self.samples.saturating_add(1);
        self.last_cycles = cycles;
        self.max_cycles = self.max_cycles.max(cycles);
        self.total_cycles = self.total_cycles.saturating_add(cycles as u64);
    }

    pub const fn samples(self) -> u32 {
        self.samples
    }

    pub const fn last_cycles(self) -> u32 {
        self.last_cycles
    }

    pub const fn max_cycles(self) -> u32 {
        self.max_cycles
    }

    pub const fn total_cycles(self) -> u64 {
        self.total_cycles
    }

    pub const fn mean_milli_cycles(self) -> u64 {
        if self.samples == 0 {
            return 0;
        }

        self.total_cycles.saturating_mul(1_000) / self.samples as u64
    }

    pub const fn snapshot(self) -> CycleStatsSnapshot {
        CycleStatsSnapshot {
            samples: self.samples,
            last_cycles: self.last_cycles,
            max_cycles: self.max_cycles,
            mean_milli_cycles: self.mean_milli_cycles(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CycleStatsSnapshot {
    pub samples: u32,
    pub last_cycles: u32,
    pub max_cycles: u32,
    pub mean_milli_cycles: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoopSample {
    start_cycles: u32,
}

impl LoopSample {
    pub const fn start_cycles(self) -> u32 {
        self.start_cycles
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoopBenchmark {
    last_start_cycles: Option<u32>,
    period: CycleStats,
    execution: CycleStats,
}

impl LoopBenchmark {
    pub const fn new() -> Self {
        Self {
            last_start_cycles: None,
            period: CycleStats::new(),
            execution: CycleStats::new(),
        }
    }

    pub fn start(&mut self, now_cycles: u32) -> LoopSample {
        if let Some(last_start_cycles) = self.last_start_cycles {
            self.period
                .record(now_cycles.wrapping_sub(last_start_cycles));
        }

        self.last_start_cycles = Some(now_cycles);

        LoopSample {
            start_cycles: now_cycles,
        }
    }

    pub fn finish(&mut self, sample: LoopSample, now_cycles: u32) {
        self.execution
            .record(now_cycles.wrapping_sub(sample.start_cycles));
    }

    pub const fn period(self) -> CycleStats {
        self.period
    }

    pub const fn execution(self) -> CycleStats {
        self.execution
    }

    pub const fn snapshot(self) -> LoopBenchmarkSnapshot {
        LoopBenchmarkSnapshot {
            period: self.period.snapshot(),
            execution: self.execution.snapshot(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoopBenchmarkSnapshot {
    pub period: CycleStatsSnapshot,
    pub execution: CycleStatsSnapshot,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BenchmarkReport {
    pub fast: LoopBenchmarkSnapshot,
    pub main: LoopBenchmarkSnapshot,
}

impl BenchmarkReport {
    pub const fn from_loops(fast: LoopBenchmark, main: LoopBenchmark) -> Self {
        Self {
            fast: fast.snapshot(),
            main: main.snapshot(),
        }
    }

    pub const fn mean_fast_loop_period_milli_cycles(self) -> u64 {
        self.fast.period.mean_milli_cycles
    }

    pub const fn t_period_fastloop(self) -> u32 {
        self.fast.period.last_cycles
    }

    pub const fn max_fast_loop_period_cycles(self) -> u32 {
        self.fast.period.max_cycles
    }

    pub const fn mean_fast_loop_cycles_milli_cycles(self) -> u64 {
        self.fast.execution.mean_milli_cycles
    }

    pub const fn t_exec_fastloop(self) -> u32 {
        self.fast.execution.last_cycles
    }

    pub const fn max_fast_loop_cycles(self) -> u32 {
        self.fast.execution.max_cycles
    }

    pub const fn mean_main_loop_period_milli_cycles(self) -> u64 {
        self.main.period.mean_milli_cycles
    }

    pub const fn t_period_mainloop(self) -> u32 {
        self.main.period.last_cycles
    }

    pub const fn max_main_loop_period_cycles(self) -> u32 {
        self.main.period.max_cycles
    }

    pub const fn mean_main_loop_cycles_milli_cycles(self) -> u64 {
        self.main.execution.mean_milli_cycles
    }

    pub const fn t_exec_mainloop(self) -> u32 {
        self.main.execution.last_cycles
    }

    pub const fn max_main_loop_cycles(self) -> u32 {
        self.main.execution.max_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_stats_track_max_total_and_fixed_point_mean() {
        let mut stats = CycleStats::new();

        assert_eq!(stats.mean_milli_cycles(), 0);

        stats.record(7);
        stats.record(10);

        assert_eq!(stats.samples(), 2);
        assert_eq!(stats.last_cycles(), 10);
        assert_eq!(stats.max_cycles(), 10);
        assert_eq!(stats.total_cycles(), 17);
        assert_eq!(stats.mean_milli_cycles(), 8_500);
        assert_eq!(
            stats.snapshot(),
            CycleStatsSnapshot {
                samples: 2,
                last_cycles: 10,
                max_cycles: 10,
                mean_milli_cycles: 8_500,
            }
        );
    }

    #[test]
    fn loop_benchmark_tracks_execution_and_periods() {
        let mut benchmark = LoopBenchmark::new();

        let first = benchmark.start(100);
        benchmark.finish(first, 130);
        let second = benchmark.start(200);
        benchmark.finish(second, 245);

        assert_eq!(benchmark.execution().samples(), 2);
        assert_eq!(benchmark.execution().max_cycles(), 45);
        assert_eq!(benchmark.execution().mean_milli_cycles(), 37_500);
        assert_eq!(benchmark.period().samples(), 1);
        assert_eq!(benchmark.period().max_cycles(), 100);
    }

    #[test]
    fn loop_benchmark_uses_wrapping_cycle_differences() {
        let mut benchmark = LoopBenchmark::new();

        let first = benchmark.start(u32::MAX - 9);
        benchmark.finish(first, 5);
        let second = benchmark.start(20);
        benchmark.finish(second, 35);

        assert_eq!(benchmark.execution().samples(), 2);
        assert_eq!(benchmark.execution().max_cycles(), 15);
        assert_eq!(benchmark.execution().mean_milli_cycles(), 15_000);
        assert_eq!(benchmark.period().samples(), 1);
        assert_eq!(benchmark.period().max_cycles(), 30);
    }

    #[test]
    fn benchmark_report_exposes_script_comparable_fields() {
        let mut fast = LoopBenchmark::new();
        let mut main = LoopBenchmark::new();

        let fast_sample = fast.start(100);
        fast.finish(fast_sample, 810);
        fast.start(3_500);

        let main_sample = main.start(1_000);
        main.finish(main_sample, 4_555);
        main.start(18_000);

        let report = BenchmarkReport::from_loops(fast, main);

        assert_eq!(report.t_exec_fastloop(), 710);
        assert_eq!(report.max_fast_loop_cycles(), 710);
        assert_eq!(report.mean_fast_loop_cycles_milli_cycles(), 710_000);
        assert_eq!(report.t_period_fastloop(), 3_400);
        assert_eq!(report.max_fast_loop_period_cycles(), 3_400);
        assert_eq!(report.mean_fast_loop_period_milli_cycles(), 3_400_000);
        assert_eq!(report.t_exec_mainloop(), 3_555);
        assert_eq!(report.max_main_loop_cycles(), 3_555);
        assert_eq!(report.mean_main_loop_cycles_milli_cycles(), 3_555_000);
        assert_eq!(report.t_period_mainloop(), 17_000);
        assert_eq!(report.max_main_loop_period_cycles(), 17_000);
        assert_eq!(report.mean_main_loop_period_milli_cycles(), 17_000_000);
    }
}
