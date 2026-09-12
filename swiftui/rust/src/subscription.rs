use std::pin::Pin;

use futures::{Stream, StreamExt as _};
use tokio::sync::{watch, Mutex};

pub(crate) struct CancellableStream<T> {
    stream: Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>,
    cancelled: watch::Sender<bool>,
}

impl<T> CancellableStream<T> {
    pub(crate) fn new(stream: impl Stream<Item = T> + Send + 'static) -> Self {
        Self {
            stream: Mutex::new(Box::pin(stream)),
            cancelled: watch::channel(false).0,
        }
    }

    pub(crate) async fn next(&self) -> Option<T> {
        let mut cancelled = self.cancelled.subscribe();
        if *cancelled.borrow() {
            return None;
        }
        tokio::select! {
            biased;
            _ = cancelled.changed() => None,
            value = async { self.stream.lock().await.next().await } => value,
        }
    }

    pub(crate) fn cancel(&self) {
        // A retained state wakes every waiter, including calls queued on the stream lock.
        self.cancelled.send_replace(true);
    }
}

macro_rules! subscription {
    ($name:ident, $event:ty) => {
        #[derive(uniffi::Object)]
        pub struct $name {
            inner: crate::subscription::CancellableStream<$event>,
        }

        #[uniffi::export(async_runtime = "tokio")]
        impl $name {
            pub async fn next(&self) -> Option<$event> {
                self.inner.next().await
            }

            pub fn cancel(&self) {
                self.inner.cancel();
            }
        }
    };
}

pub(crate) use subscription;

#[cfg(test)]
mod tests {
    use futures::{poll, stream};

    use super::CancellableStream;

    #[tokio::test]
    async fn cancellation_releases_pending_and_queued_reads_and_stays_cancelled() {
        let subscription = CancellableStream::new(stream::pending::<u64>());
        let mut first = Box::pin(subscription.next());
        let mut queued = Box::pin(subscription.next());
        assert!(poll!(&mut first).is_pending());
        assert!(poll!(&mut queued).is_pending());

        subscription.cancel();
        assert_eq!(poll!(&mut first), std::task::Poll::Ready(None));
        assert_eq!(poll!(&mut queued), std::task::Poll::Ready(None));
        assert_eq!(subscription.next().await, None);
        subscription.cancel();
        assert_eq!(subscription.next().await, None);
    }

    #[tokio::test]
    async fn stream_values_and_typed_failures_are_delivered_in_order_until_closed() {
        let subscription = CancellableStream::new(stream::iter([
            Ok(42),
            Err(crate::error::SwiftClientError::Unauthorized),
        ]));
        assert!(matches!(subscription.next().await, Some(Ok(42))));
        assert!(matches!(
            subscription.next().await,
            Some(Err(crate::error::SwiftClientError::Unauthorized))
        ));
        assert!(subscription.next().await.is_none());
    }

    #[tokio::test]
    async fn cancellation_discards_buffered_events() {
        let subscription = CancellableStream::new(stream::iter([42]));
        subscription.cancel();
        assert_eq!(subscription.next().await, None);
    }
}
