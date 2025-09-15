//! Simple Circuit Breaker
//!
//! Minimal implementation.

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitResult {
    Rejected,
    Failed,
    Succeeded,
}

/// Simple circuit breaker
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u8,
    failure_threshold: u8,
    half_open_attempts: u8,
    half_open_threshold: u8,
}

impl CircuitBreaker {
    /// Create new circuit breaker with custom thresholds
    pub fn new(failure_threshold: u8, half_open_threshold: u8) -> Self {
        assert!(failure_threshold > 0);
        assert!(half_open_threshold > 0);

        CircuitBreaker {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            half_open_attempts: 0,
            half_open_threshold,
        }
    }

    /// Execute operation through circuit breaker
    pub fn call<F, R>(&mut self, f: F) -> CircuitResult
    where
        F: FnOnce() -> Result<R, ()>,
    {
        match self.state {
            CircuitState::Closed => {
                assert!(self.failure_count < self.failure_threshold);
                assert!(self.half_open_attempts == 0);

                let result = f();
                match result {
                    Ok(_) => CircuitResult::Succeeded,
                    Err(_) => {
                        self.failure_count += 1;
                        if self.failure_count == self.failure_threshold {
                            self.state = CircuitState::Open;
                        }
                        CircuitResult::Failed
                    }
                }
            }
            CircuitState::Open => {
                assert!(self.failure_count == self.failure_threshold);
                assert!(self.half_open_attempts < self.half_open_threshold);

                self.half_open_attempts += 1;
                if self.half_open_attempts == self.half_open_threshold {
                    self.state = CircuitState::HalfOpen;
                    self.half_open_attempts = 0;
                }
                CircuitResult::Rejected
            }
            CircuitState::HalfOpen => {
                assert!(self.failure_count == self.failure_threshold);
                assert!(self.half_open_attempts < self.half_open_threshold);

                let result = f();
                match result {
                    Ok(_) => {
                        self.state = CircuitState::Closed;
                        self.failure_count = 0;
                        CircuitResult::Succeeded
                    }
                    Err(_) => {
                        self.state = CircuitState::Open;
                        self.failure_count = 0;
                        self.half_open_attempts = 0;
                        CircuitResult::Failed
                    }
                }
            }
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_zero_failure_threshold_panics() {
        CircuitBreaker::new(0, 1);
    }

    #[test]
    #[should_panic]
    fn test_zero_half_open_threshold_panics() {
        CircuitBreaker::new(1, 0);
    }

    #[test]
    #[should_panic]
    fn test_both_zero_thresholds_panic() {
        CircuitBreaker::new(0, 0);
    }

    #[test]
    fn test_closed_success() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Succeeded);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_closed_failure_stays_closed() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_closed_to_open() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_open_rejects_calls() {
        let mut cb = CircuitBreaker::new(2, 2);
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
    fn test_open_to_halfopen() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Rejected);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_halfopen_success_to_closed() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Rejected);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Succeeded);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_halfopen_failure_to_open() {
        let mut cb = CircuitBreaker::new(2, 1);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);

        let result = cb.call(|| Ok::<(), ()>(()));
        assert_eq!(result, CircuitResult::Rejected);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        let result = cb.call(|| Err::<(), ()>(()));
        assert_eq!(result, CircuitResult::Failed);
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
