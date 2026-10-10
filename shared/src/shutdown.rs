//! Graceful shutdown of a service's background threads (Kafka listener, outbox relay).
//!
//! On SIGTERM actix stops accepting requests and finishes the in-flight ones; the starter then
//! calls [`Workers::stop`], which signals [`Shutdown`] and waits for the threads to finish their
//! current message or batch. Kubernetes' `terminationGracePeriodSeconds` must cover both.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Cloneable stop flag shared by the background threads.
#[derive(Clone, Default)]
pub struct Shutdown {
    triggered: Arc<AtomicBool>,
}

impl Shutdown {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::SeqCst);
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    /// Sleeps for `duration`, waking up early on shutdown. Returns `false` if shutdown was
    /// triggered, so loops can write `while shutdown.sleep(interval) { .. }`.
    pub fn sleep(&self, duration: Duration) -> bool {
        const SLICE: Duration = Duration::from_millis(100);
        let deadline = Instant::now() + duration;

        loop {
            if self.is_triggered() {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            std::thread::sleep(SLICE.min(deadline - now));
        }
    }
}

/// The background threads of a service, stopped together.
pub struct Workers {
    shutdown: Shutdown,
    threads: Vec<(&'static str, JoinHandle<()>)>,
}

impl Workers {
    pub fn new(shutdown: Shutdown) -> Self {
        Self {
            shutdown,
            threads: vec![],
        }
    }

    pub fn add(&mut self, name: &'static str, thread: JoinHandle<()>) {
        self.threads.push((name, thread));
    }

    /// Signals shutdown and waits up to `timeout` for every thread to finish.
    pub fn stop(self, timeout: Duration) {
        self.shutdown.trigger();
        let deadline = Instant::now() + timeout;

        for (name, thread) in self.threads {
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            if thread.is_finished() {
                if thread.join().is_err() {
                    tracing::error!("{name} panicked while stopping");
                } else {
                    tracing::info!("{name} stopped");
                }
            } else {
                tracing::warn!("{name} did not stop within {timeout:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_returns_early_once_triggered() {
        let shutdown = Shutdown::new();
        let trigger = shutdown.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            trigger.trigger();
        });

        let started = Instant::now();
        assert!(!shutdown.sleep(Duration::from_secs(10)));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn sleep_runs_to_the_end_without_shutdown() {
        assert!(Shutdown::new().sleep(Duration::from_millis(20)));
    }

    #[test]
    fn stop_waits_for_threads_that_watch_the_flag() {
        let shutdown = Shutdown::new();
        let mut workers = Workers::new(shutdown.clone());
        let watched = shutdown.clone();
        let thread = std::thread::spawn(move || while watched.sleep(Duration::from_secs(1)) {});
        workers.add("worker", thread);

        let started = Instant::now();
        workers.stop(Duration::from_secs(5));

        assert!(shutdown.is_triggered());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
