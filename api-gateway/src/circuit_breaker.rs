//! Count-based circuit breaker with the same semantics as the resilience4j
//! defaults the Spring gateway used: once the sliding window is full and the
//! failure rate reaches the threshold, the circuit opens; after the wait it lets
//! a few trial calls through (half-open) and closes again only if they succeed.
//!
//! The state is published as `gateway_circuit_breaker_state{breaker}`: 0 closed,
//! 1 half-open, 2 open.

use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct CircuitBreakerConfig {
    pub sliding_window_size: usize,
    pub permitted_calls_in_half_open: usize,
    /// Percentage (0-100) of failed calls that opens the circuit.
    pub failure_rate_threshold: u8,
    pub wait_duration_in_open: Duration,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Closed,
    Open { until: Instant },
    HalfOpen { started: usize },
}

impl State {
    fn metric_value(&self) -> f64 {
        match self {
            State::Closed => 0.0,
            State::HalfOpen { .. } => 1.0,
            State::Open { .. } => 2.0,
        }
    }
}

#[derive(Debug)]
struct Inner {
    state: State,
    /// `true` = failure. Holds the closed-state window, or the half-open trial results.
    outcomes: VecDeque<bool>,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    inner: Mutex<Inner>,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let breaker = Self {
            name: name.into(),
            config,
            inner: Mutex::new(Inner {
                state: State::Closed,
                outcomes: VecDeque::new(),
            }),
        };
        breaker.publish(&State::Closed);
        breaker
    }

    fn publish(&self, state: &State) {
        metrics::gauge!("gateway_circuit_breaker_state", "breaker" => self.name.clone())
            .set(state.metric_value());
    }

    /// Returns `None` when the call must be short-circuited.
    pub fn try_acquire(&self) -> Option<Permit<'_>> {
        self.try_acquire_at(Instant::now()).then_some(Permit {
            breaker: self,
            recorded: false,
        })
    }

    fn try_acquire_at(&self, now: Instant) -> bool {
        let mut inner = self.inner.lock().unwrap();
        match inner.state {
            State::Closed => true,
            State::Open { until } if now < until => false,
            State::Open { .. } => {
                tracing::info!(breaker = %self.name, "circuit half-open");
                inner.state = State::HalfOpen { started: 1 };
                inner.outcomes.clear();
                self.publish(&inner.state);
                true
            }
            State::HalfOpen { ref mut started } => {
                if *started < self.config.permitted_calls_in_half_open {
                    *started += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Frees a half-open trial slot whose call was cancelled before it had an outcome.
    fn release(&self) {
        let mut inner = self.inner.lock().unwrap();
        if let State::HalfOpen { ref mut started } = inner.state {
            *started = started.saturating_sub(1);
        }
    }

    fn record_at(&self, success: bool, now: Instant) {
        let mut inner = self.inner.lock().unwrap();
        // A call admitted before the circuit opened can finish while it is open; ignore it.
        if matches!(inner.state, State::Open { .. }) {
            return;
        }

        inner.outcomes.push_back(!success);
        let window = match inner.state {
            State::Closed => self.config.sliding_window_size,
            _ => self.config.permitted_calls_in_half_open,
        };
        while inner.outcomes.len() > window {
            inner.outcomes.pop_front();
        }
        if inner.outcomes.len() < window {
            return;
        }

        let failures = inner.outcomes.iter().filter(|failed| **failed).count();
        let failure_rate = failures * 100 / window;
        if failure_rate >= self.config.failure_rate_threshold as usize {
            tracing::warn!(breaker = %self.name, failure_rate, "circuit opened");
            inner.state = State::Open {
                until: now + self.config.wait_duration_in_open,
            };
            inner.outcomes.clear();
            self.publish(&inner.state);
        } else if matches!(inner.state, State::HalfOpen { .. }) {
            tracing::info!(breaker = %self.name, "circuit closed");
            inner.state = State::Closed;
            inner.outcomes.clear();
            self.publish(&inner.state);
        }
    }
}

/// An admitted call. Dropping it without recording (e.g. the client hung up)
/// counts as neither success nor failure.
pub struct Permit<'a> {
    breaker: &'a CircuitBreaker,
    recorded: bool,
}

impl Permit<'_> {
    pub fn record(mut self, success: bool) {
        self.recorded = true;
        self.breaker.record_at(success, Instant::now());
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        if !self.recorded {
            self.breaker.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                sliding_window_size: 4,
                permitted_calls_in_half_open: 2,
                failure_rate_threshold: 50,
                wait_duration_in_open: Duration::from_secs(10),
            },
        )
    }

    fn call(cb: &CircuitBreaker, success: bool, now: Instant) -> bool {
        let admitted = cb.try_acquire_at(now);
        if admitted {
            cb.record_at(success, now);
        }
        admitted
    }

    #[test]
    fn stays_closed_until_window_is_full() {
        let cb = breaker();
        let now = Instant::now();
        for _ in 0..3 {
            assert!(call(&cb, false, now));
        }
        assert!(cb.try_acquire_at(now));
    }

    #[test]
    fn opens_when_failure_rate_reaches_threshold() {
        let cb = breaker();
        let now = Instant::now();
        for success in [true, true, false, false] {
            call(&cb, success, now);
        }
        assert!(!cb.try_acquire_at(now));
    }

    #[test]
    fn stays_closed_below_threshold() {
        let cb = breaker();
        let now = Instant::now();
        for success in [true, true, true, false, true] {
            call(&cb, success, now);
        }
        assert!(cb.try_acquire_at(now));
    }

    #[test]
    fn half_open_limits_trial_calls_and_closes_on_success() {
        let cb = breaker();
        let now = Instant::now();
        for _ in 0..4 {
            call(&cb, false, now);
        }
        let later = now + Duration::from_secs(11);

        assert!(cb.try_acquire_at(later));
        assert!(cb.try_acquire_at(later));
        assert!(
            !cb.try_acquire_at(later),
            "only two trial calls are permitted"
        );

        cb.record_at(true, later);
        cb.record_at(true, later);
        assert_eq!(cb.inner.lock().unwrap().state, State::Closed);
    }

    #[test]
    fn dropped_permit_frees_half_open_slot() {
        let cb = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                wait_duration_in_open: Duration::ZERO,
                ..breaker().config
            },
        );
        let now = Instant::now();
        for _ in 0..4 {
            call(&cb, false, now);
        }

        let first = cb.try_acquire().expect("first trial call");
        let second = cb.try_acquire().expect("second trial call");
        assert!(cb.try_acquire().is_none());

        drop(second);
        assert!(cb.try_acquire().is_some());
        first.record(true);
    }

    #[test]
    fn half_open_reopens_on_failure() {
        let cb = breaker();
        let now = Instant::now();
        for _ in 0..4 {
            call(&cb, false, now);
        }
        let later = now + Duration::from_secs(11);
        call(&cb, true, later);
        call(&cb, false, later);

        assert!(!cb.try_acquire_at(later));
    }
}
