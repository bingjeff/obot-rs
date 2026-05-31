#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CycleStats {
    samples: u32,
    max_cycles: u32,
    total_cycles: u64,
}

impl CycleStats {
    pub const fn new() -> Self {
        Self {
            samples: 0,
            max_cycles: 0,
            total_cycles: 0,
        }
    }

    pub fn record(&mut self, cycles: u32) {
        self.samples = self.samples.saturating_add(1);
        self.max_cycles = self.max_cycles.max(cycles);
        self.total_cycles = self.total_cycles.saturating_add(cycles as u64);
    }

    pub const fn samples(self) -> u32 {
        self.samples
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
        assert_eq!(stats.max_cycles(), 10);
        assert_eq!(stats.total_cycles(), 17);
        assert_eq!(stats.mean_milli_cycles(), 8_500);
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
}
