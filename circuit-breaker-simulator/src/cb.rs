//! Circuit Breaker core types and trait

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

/// Circuit Breaker trait
pub trait CircuitBreaker {
    fn call<F, R>(&mut self, f: F) -> CircuitResult
    where
        F: FnOnce() -> Result<R, ()>;

    fn state(&self) -> CircuitState;
}
