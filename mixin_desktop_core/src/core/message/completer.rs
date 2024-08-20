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
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_transaction() {
        let t = Completer::<String>::default();

        let t_clone = t.clone();
        tokio::spawn(async move {
            println!("start spawn");
            let result = t_clone.await;
            println!("completed: {:?}", result);
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("run after 100ms");
        t.complete(Ok("haha".to_string()));
        println!("run after complete");
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("run after 200ms");
    }
}
