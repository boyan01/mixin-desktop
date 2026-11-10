//! Browser task delivery: the page has one thread, so business futures run on
//! the engine's executor instead of a separate business runtime.
use std::future::Future;
use std::time::Duration;

use mixin_desktop_api::{ClientError, ClientResult};
use nsh_core::prelude::timer;

/// Browser builds have one thread, so business work need not be `Send`.
pub(crate) trait MaybeSend {}
impl<T> MaybeSend for T {}

#[track_caller]
pub(crate) fn run<T: 'static>(
    future: impl Future<Output = ClientResult<T>> + 'static,
) -> impl Future<Output = ClientResult<T>> {
    let caller = std::panic::Location::caller();
    // The awaiting UI task owns the future, so disposing it cancels the work.
    async move { logged(caller, future.await) }
}

pub(crate) fn observe<S: futures::Stream + 'static>(
    source: S,
) -> impl futures::Stream<Item = S::Item> {
    source
}

pub(crate) fn throttled<S: futures::Stream + 'static>(
    source: S,
    interval: Duration,
) -> impl futures::Stream<Item = ()> {
    use futures::{FutureExt as _, StreamExt as _};
    futures::stream::unfold(Box::pin(source), move |mut source| async move {
        source.next().await?;
        timer(interval).await;
        while source.next().now_or_never().flatten().is_some() {}
        Some(((), source))
    })
}

// A user-committed write must finish even if navigation disposes its view.
#[track_caller]
pub(crate) fn commit<T: 'static>(
    future: impl Future<Output = ClientResult<T>> + 'static,
) -> impl Future<Output = ClientResult<T>> {
    let caller = std::panic::Location::caller();
    let (sender, receiver) = futures::channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = sender.send(future.await);
    });
    async move {
        logged(
            caller,
            receiver
                .await
                .map_err(|_| ClientError::Internal("Command was cancelled".into()))
                .and_then(|result| result),
        )
    }
}

pub(crate) fn retain_account(_account: std::sync::Arc<mixin_desktop_api::AccountClient>) {}

pub(crate) fn release_account() {}

pub(crate) fn stop() {}

fn logged<T>(
    caller: &'static std::panic::Location<'static>,
    result: ClientResult<T>,
) -> ClientResult<T> {
    if let Err(error) = &result {
        log::warn!(target: "ui", "{}:{}: {error}", caller.file(), caller.line());
    }
    result
}
