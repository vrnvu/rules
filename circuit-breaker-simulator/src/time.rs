use crate::{CircuitBreaker, CircuitResult, CircuitState};
use std::time::{Duration, Instant};

pub trait Clock {
    fn now(&self) -> Instant;
}

#[derive(Debug, Clone, Copy)]
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug)]
pub struct TimeCB<C: Clock = RealClock> {
    clock: C,
    state: CircuitState,
    open_timeout: Duration,
    open_at: Option<Instant>,
    closed_failures: u8,
    closed_failures_threshold: u8,
    half_open_probes: u8,
    half_open_probes_threshold: u8,
}

impl TimeCB<RealClock> {
    pub fn new(
        open_timeout: Duration,
        half_open_probes_threshold: u8,
        closed_failures_threshold: u8,
    ) -> Self {
        assert!(open_timeout > Duration::from_millis(0));
        assert!(half_open_probes_threshold > 0);
        assert!(closed_failures_threshold > 0);

        Self::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            RealClock,
        )
    }
}

impl<C: Clock> TimeCB<C> {
    pub fn with_clock(
        open_timeout: Duration,
        half_open_probes_threshold: u8,
        closed_failures_threshold: u8,
        clock: C,
    ) -> Self {
        assert!(open_timeout > Duration::from_millis(0));
        assert!(half_open_probes_threshold > 0);
        assert!(closed_failures_threshold > 0);

        TimeCB {
            clock,
            state: CircuitState::Closed,
            open_at: None,
            half_open_probes: 0,
            closed_failures: 0,
            closed_failures_threshold,
            open_timeout,
            half_open_probes_threshold,
        }
    }
}

impl<C: Clock> CircuitBreaker for TimeCB<C> {
    fn call<F, R>(&mut self, f: F) -> CircuitResult
    where
        F: FnOnce() -> Result<R, ()>,
    {
        match self.state {
            CircuitState::Closed => {
                assert!(self.closed_failures < self.closed_failures_threshold);
                assert!(self.half_open_probes == 0);
                assert!(self.open_at.is_none());

                let result = f();
                match result {
                    Ok(_) => {
                        self.closed_failures = 0;
                        CircuitResult::Succeeded
                    }
                    Err(_) => {
                        self.closed_failures += 1;
                        if self.closed_failures == self.closed_failures_threshold {
                            self.state = CircuitState::Open;
                            self.open_at = Some(self.clock.now());
                        }
                        CircuitResult::Failed
                    }
                }
            }
            CircuitState::Open => {
                assert!(self.closed_failures == self.closed_failures_threshold);
                assert!(self.half_open_probes == 0);
                assert!(self.open_at.is_some());

                if self.open_at.unwrap() + self.open_timeout <= self.clock.now() {
                    self.state = CircuitState::HalfOpen;
                    self.half_open_probes = 0;

                    let result = f();
                    match result {
                        Ok(_) => {
                            self.state = CircuitState::Closed;
                            self.closed_failures = 0;
                            self.open_at = None;
                            self.half_open_probes = 0;
                            CircuitResult::Succeeded
                        }
                        Err(_) => {
                            self.half_open_probes += 1;
                            if self.half_open_probes == self.half_open_probes_threshold {
                                self.state = CircuitState::Open;
                                self.half_open_probes = 0;
                                self.open_at = Some(self.clock.now());
                            }
                            CircuitResult::Failed
                        }
                    }
                } else {
                    CircuitResult::Rejected
                }
            }
            CircuitState::HalfOpen => {
                assert!(self.closed_failures == self.closed_failures_threshold);
                assert!(self.half_open_probes < self.half_open_probes_threshold);
                assert!(self.open_at.is_some());
                assert!(self.open_at.unwrap() + self.open_timeout <= self.clock.now());

                let result = f();
                match result {
                    Ok(_) => {
                        self.state = CircuitState::Closed;
                        self.closed_failures = 0;
                        self.open_at = None;
                        self.half_open_probes = 0;
                        CircuitResult::Succeeded
                    }
                    Err(_) => {
                        self.half_open_probes += 1;
                        if self.half_open_probes == self.half_open_probes_threshold {
                            self.state = CircuitState::Open;
                            self.half_open_probes = 0;
                            self.open_at = Some(self.clock.now());
                        }
                        CircuitResult::Failed
                    }
                }
            }
        }
    }

    fn state(&self) -> CircuitState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;
    use std::rc::Rc;

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

    #[test]
    #[should_panic]
    fn test_zero_open_timeout_panics() {
        let open_timeout = Duration::from_millis(0);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let _ = TimeCB::new(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
        );
    }

    #[test]
    #[should_panic]
    fn test_zero_half_open_probes_threshold_panics() {
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 0;
        let closed_failures_threshold = 2;
        let _ = TimeCB::new(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
        );
    }

    #[test]
    #[should_panic]
    fn test_zero_closed_failures_threshold_panics() {
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 0;
        let _ = TimeCB::new(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
        );
    }

    #[test]
    fn test_closed_success() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Succeeded);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_closed_failure_stays_closed() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_closed_to_open() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_open_rejects_calls_immediately() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Rejected);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_open_rejects_until_timeout_then_allows_half_open_call() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        clock.tick();

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Succeeded);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_success_closes_breaker() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        clock.tick();

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Succeeded);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_failure_opens_breaker() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let open_timeout = Duration::from_millis(1);
        let half_open_probes_threshold = 1;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            open_timeout,
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_half_open_respects_probes_threshold() {
        let start = Instant::now();
        let clock = TestClock::new(start);
        let half_open_probes_threshold = 2;
        let closed_failures_threshold = 2;
        let mut cb = TimeCB::with_clock(
            Duration::from_millis(1),
            half_open_probes_threshold,
            closed_failures_threshold,
            clock.clone(),
        );
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        clock.tick();

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Rejected);
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
