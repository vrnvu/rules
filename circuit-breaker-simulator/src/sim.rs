//! Simulation harness for circuit breakers

#[cfg(test)]
mod tests {
    use crate::cb::CircuitBreaker;
    use crate::count::CountCB;
    use crate::time::{Clock, TimeCB};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StepCount {
        Success,
        Failure,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StepTime {
        Success,
        Failure,
        Tick,
    }

    #[derive(Debug, Clone)]
    struct TestClock {
        now: Rc<Cell<Instant>>,
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            self.now.get()
        }
    }

    impl TestClock {
        const TICK: Duration = Duration::from_millis(1);

        fn new(start: Instant) -> Self {
            Self {
                now: Rc::new(Cell::new(start)),
            }
        }

        fn tick(&self) {
            self.now.set(self.now.get() + Self::TICK);
        }
    }

    fn generate_random_steps_count(seed: u64, count: usize) -> Vec<StepCount> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut steps = Vec::with_capacity(count);

        for _ in 0..count {
            let choice = if rng.random_range(0..2) == 0 {
                StepCount::Success
            } else {
                StepCount::Failure
            };
            steps.push(choice);
        }

        steps
    }

    fn generate_random_steps_time(seed: u64, count: usize) -> Vec<StepTime> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut steps = Vec::with_capacity(count);

        for _ in 0..count {
            let choice = rng.random_range(0..3);
            steps.push(match choice {
                0 => StepTime::Success,
                1 => StepTime::Failure,
                _ => StepTime::Tick,
            });
        }

        steps
    }

    #[test]
    fn test_count_cb_random_sequence() {
        let failure_threshold = 10;
        let half_open_threshold = 4;
        let seed = 42;
        let count: usize = 100_000;
        let mut cb = CountCB::new(failure_threshold, half_open_threshold);
        let steps = generate_random_steps_count(seed, count);

        for step in steps {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                cb.call(match step {
                    StepCount::Success => || Ok::<(), ()>(()),
                    StepCount::Failure => || Err::<(), ()>(()),
                })
            }));

            assert!(result.is_ok(), "Panic occurred with step: {:?}", step);
        }
    }

    #[test]
    fn test_time_cb_random_sequence() {
        let open_timeout = Duration::from_millis(5);
        let half_open_probes_threshold = 5;
        let closed_failures_threshold = 10;
        let seed = 42;
        let count = 100_000;
        let start = Instant::now();
        let clock = TestClock::new(start);
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        let steps = generate_random_steps_time(seed, count);

        for step in steps {
            match step {
                StepTime::Tick => clock.tick(),
                StepTime::Success => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        cb.call(|| Ok::<(), ()>(()))
                    }));
                    assert!(result.is_ok(), "Panic occurred with step: {:?}", step);
                }
                StepTime::Failure => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        cb.call(|| Err::<(), ()>(()))
                    }));
                    assert!(result.is_ok(), "Panic occurred with step: {:?}", step);
                }
            }
        }
    }
}
