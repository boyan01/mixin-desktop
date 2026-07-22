use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use anyhow::Result;

pub struct Completer<T, E = anyhow::Error> {
    shared_state: Arc<Mutex<SharedState<T, E>>>,
}

impl<T, E> Clone for Completer<T, E> {
    fn clone(&self) -> Self {
        Completer {
            shared_state: self.shared_state.clone(),
        }
    }
}

impl<T, E> Default for Completer<T, E> {
    fn default() -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
        }));
        Completer { shared_state }
    }
}

impl<T, E> Completer<T, E> {
    pub fn complete(&self, result: Result<T, E>) {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.result.is_some() {
            return;
        }
        shared_state.result = Some(result);
        if let Some(waker) = shared_state.waker.take() {
            waker.wake();
        }
    }
}

struct SharedState<T, E> {
    result: Option<Result<T, E>>,
    waker: Option<Waker>,
}

impl<T, E> Future for Completer<T, E> {
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if let Some(result) = shared_state.result.take() {
            Poll::Ready(result)
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn waits_for_completion_and_returns_the_first_result() {
        let completer = Completer::<String>::default();
        let task = tokio::spawn(completer.clone());

        tokio::task::yield_now().await;
        assert!(!task.is_finished());

        completer.complete(Ok("first".to_string()));
        completer.complete(Ok("second".to_string()));

        assert_eq!(task.await.unwrap().unwrap(), "first");
    }
}
