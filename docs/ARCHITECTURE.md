# Architecture

## Workspace
- Crates: `circuit-breaker-simulator`, `load-balancer-simulator`
- Shared deps via `[workspace.dependencies]` in `Cargo.toml`

## Circuit Breaker Simulator
- Public API: `CircuitBreaker` trait; concrete `CountCB`, `TimeCB`
- Invariants: documented via assertions; states: Closed, Open, HalfOpen
- Testing: unit and randomized integration-style tests

## Load Balancer Simulator
- Public API: `LoadBalancer` trait; strategies: RoundRobin, LeastConnections
- Invariants: selection rules and health tracking
- Testing: unit tests and randomized scenarios

## Rules
- `.cursor/rules/project/good-project-style.mdc` applies globally
- Language-specific rules in `assertions/`, `style/`, `testing/`

