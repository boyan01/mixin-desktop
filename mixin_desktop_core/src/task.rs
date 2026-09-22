//! Platform scheduling. Dropping a handle detaches; abort explicitly cancels.
#[cfg(target_arch = "wasm32")]
pub use browser::{spawn, JoinHandle};
#[cfg(not(target_arch = "wasm32"))]
pub use tokio::{spawn, task::JoinHandle};

#[cfg(target_arch = "wasm32")]
mod browser {
    use futures::{
        channel::oneshot,
        future::{AbortHandle, Abortable},
    };
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };
    #[derive(Debug, thiserror::Error)]
    #[error("background task cancelled")]
    pub struct JoinError;
    pub struct JoinHandle<T> {
        receiver: oneshot::Receiver<T>,
        abort: AbortHandle,
    }
    impl<T> JoinHandle<T> {
        pub fn abort(&self) {
            self.abort.abort();
        }
    }
    impl<T> Future for JoinHandle<T> {
        type Output = Result<T, JoinError>;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut self.get_mut().receiver)
                .poll(cx)
                .map(|r| r.map_err(|_| JoinError))
        }
    }
    pub fn spawn<F: Future + 'static>(future: F) -> JoinHandle<F::Output>
    where
        F::Output: 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let (abort, registration) = AbortHandle::new_pair();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(value) = Abortable::new(future, registration).await {
                let _ = sender.send(value);
            }
        });
        JoinHandle { receiver, abort }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use tokio::task::yield_now;
#[cfg(target_arch = "wasm32")]
pub async fn yield_now() {
    crate::time::sleep(std::time::Duration::ZERO).await;
}
