//! Basic Circuit Breaker
//!
//! A simple circuit breaker implementation following assertion style rules.
//! Provides basic circuit breaker functionality with state transitions and metrics.

pub mod circuit_breaker;

pub use circuit_breaker::*;
