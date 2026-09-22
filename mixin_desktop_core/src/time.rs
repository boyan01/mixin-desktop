//! Timers for the host event loop; native retains Tokio's paused-time support.
#[cfg(target_arch = "wasm32")]
pub use browser::*;
pub use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
pub use tokio::time::{interval, interval_at, sleep, timeout, Instant};
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::Duration;
    pub use web_time::Instant;
    // The future carries only a Rust channel. JS timer handles stay on the
    // owning Worker, so shared subscription futures need no unsafe Send shim.
    thread_local! {
        static TIMERS:std::cell::RefCell<std::collections::HashMap<u64,gloo_timers::callback::Timeout>>=std::cell::RefCell::new(std::collections::HashMap::new());
        static NEXT:std::cell::Cell<u64>=const{std::cell::Cell::new(0)};
    }
    struct Timer(u64);
    impl Drop for Timer {
        fn drop(&mut self) {
            TIMERS.with(|timers| {
                timers.borrow_mut().remove(&self.0);
            });
        }
    }
    pub async fn sleep(duration: Duration) {
        let mut remaining = duration.as_millis();
        loop {
            let delay = remaining.min(i32::MAX as u128) as u32;
            let (sender, receiver) = futures::channel::oneshot::channel();
            let id = NEXT.with(|next| {
                let id = next.get() + 1;
                next.set(id);
                id
            });
            let timer = gloo_timers::callback::Timeout::new(delay, move || {
                let _ = sender.send(());
            });
            TIMERS.with(|timers| {
                timers.borrow_mut().insert(id, timer);
            });
            let _timer = Timer(id);
            let _ = receiver.await;
            if remaining <= delay as u128 {
                break;
            }
            remaining -= delay as u128;
        }
    }
    #[derive(Debug, thiserror::Error)]
    #[error("deadline elapsed")]
    pub struct Elapsed;
    pub async fn timeout<F: std::future::Future>(
        duration: Duration,
        future: F,
    ) -> Result<F::Output, Elapsed> {
        let future = Box::pin(future);
        let timer = Box::pin(sleep(duration));
        match futures::future::select(future, timer).await {
            futures::future::Either::Left((value, _)) => Ok(value),
            futures::future::Either::Right(_) => Err(Elapsed),
        }
    }
    pub struct Interval {
        next: Instant,
        period: Duration,
    }
    pub fn interval(period: Duration) -> Interval {
        interval_at(Instant::now(), period)
    }
    pub fn interval_at(start: Instant, period: Duration) -> Interval {
        assert!(!period.is_zero());
        Interval {
            next: start,
            period,
        }
    }
    impl Interval {
        pub async fn tick(&mut self) -> Instant {
            sleep(self.next.saturating_duration_since(Instant::now())).await;
            let tick = self.next;
            self.next += self.period;
            tick
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant as WallInstant;
