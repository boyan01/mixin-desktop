use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use mixin_desktop_api::{AccountClient, ClientError, ClientResult};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_util::task::{AbortOnDropHandle, TaskTracker};

// Outlive NSH's engine so quitting can drain the business runtime.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static COMMITS: OnceLock<TaskTracker> = OnceLock::new();
static ACCOUNT: Mutex<Option<Arc<AccountClient>>> = Mutex::new(None);
static SHUTDOWN: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("create the business runtime"))
}

/// Logs a failure where the UI asked for the work: views turn errors into a
/// state such as "failed", which alone would leave nothing to diagnose.
fn logged<T>(
    caller: &'static std::panic::Location<'static>,
    result: ClientResult<T>,
) -> ClientResult<T> {
    if let Err(error) = &result {
        log::warn!(
            target: "ui",
            "{}:{}: {error}",
            caller.file(),
            caller.line()
        );
    }
    result
}

#[track_caller]
pub(crate) fn run<T: Send + 'static>(
    future: impl Future<Output = ClientResult<T>> + Send + 'static,
) -> impl Future<Output = ClientResult<T>> {
    let caller = std::panic::Location::caller();
    // Disposing the UI owner must cancel its background work too.
    let task = AbortOnDropHandle::new(runtime().spawn(future));
    async move {
        logged(
            caller,
            task.await
                .map_err(|error| ClientError::Internal(format!("Background task failed: {error}")))
                .and_then(|result| result),
        )
    }
}

/// Polls business subscriptions on the business runtime, with bounded delivery.
/// Dropping the UI subscription aborts its producer and releases its source.
pub(crate) fn observe<S>(source: S) -> impl futures::Stream<Item = S::Item>
where
    S: futures::Stream + Send + 'static,
    S::Item: Send + 'static,
{
    use futures::StreamExt as _;
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let task = AbortOnDropHandle::new(runtime().spawn(async move {
        futures::pin_mut!(source);
        while let Some(item) = source.next().await {
            if sender.send(item).await.is_err() {
                break;
            }
        }
    }));
    futures::stream::unfold((receiver, task), |(mut receiver, task)| async move {
        receiver.recv().await.map(|item| (item, (receiver, task)))
    })
}

/// Wakes once per burst of `source` events, so a busy sync cannot keep restarting
/// debounced UI queries or queue one reload per message.
pub(crate) fn throttled<S>(source: S, interval: Duration) -> impl futures::Stream<Item = ()>
where
    S: futures::Stream + Send + 'static,
    S::Item: Send,
{
    use futures::{FutureExt as _, StreamExt as _};
    observe(futures::stream::unfold(
        Box::pin(source),
        move |mut source| async move {
            source.next().await?;
            tokio::time::sleep(interval).await;
            while source.next().now_or_never().flatten().is_some() {}
            Some(((), source))
        },
    ))
}

// A user-committed write must finish even if navigation disposes its view.
#[track_caller]
pub(crate) fn commit<T: Send + 'static>(
    future: impl Future<Output = ClientResult<T>> + Send + 'static,
) -> impl Future<Output = ClientResult<T>> {
    let caller = std::panic::Location::caller();
    let task = COMMITS
        .get_or_init(TaskTracker::new)
        .spawn_on(future, runtime().handle());
    async move {
        logged(
            caller,
            task.await
                .map_err(|error| ClientError::Internal(format!("Command failed: {error}")))
                .and_then(|result| result),
        )
    }
}

pub(crate) fn retain_account(account: Arc<AccountClient>) {
    *ACCOUNT.lock().expect("account lock") = Some(account);
}

pub(crate) fn release_account() {
    ACCOUNT.lock().expect("account lock").take();
}

pub(crate) fn stop() {
    if let Some(account) = ACCOUNT.lock().expect("account lock").take() {
        *SHUTDOWN.lock().expect("shutdown lock") = Some(runtime().spawn(async move {
            if let Some(commits) = COMMITS.get() {
                commits.close();
                commits.wait().await;
            }
            account.shutdown().await;
            log::info!("NSH account shutdown completed");
        }));
    }
}

/// Routes terminal termination through the native loop so committed work can drain.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn quit_on_signal(quit: fn()) -> std::io::Result<AbortOnDropHandle<()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let _entered = runtime().enter();
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    Ok(AbortOnDropHandle::new(runtime().spawn(async move {
        tokio::select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
        }
        quit();
    })))
}

/// Drains account shutdown after the native event loop returns.
pub fn finish_shutdown() {
    stop();
    let task = SHUTDOWN.lock().expect("shutdown lock").take();
    runtime().block_on(async {
        let drain = async {
            // Process settings can be committed from the sign-in screen too.
            // Their lifetime must not depend on retaining a signed-in account.
            if let Some(commits) = COMMITS.get() {
                commits.close();
                commits.wait().await;
            }
            if let Some(task) = task {
                let _ = AbortOnDropHandle::new(task).await;
            }
        };
        if tokio::time::timeout(Duration::from_secs(10), drain)
            .await
            .is_err()
        {
            log::warn!("Timed out waiting for runtime shutdown");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    struct NotifyOnDrop(Option<oneshot::Sender<()>>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[tokio::test]
    async fn disposing_an_awaiting_ui_task_cancels_its_business_future() {
        let (started, ready) = oneshot::channel();
        let (cancelled, dropped) = oneshot::channel();
        let task = tokio::spawn(run(async move {
            let _guard = NotifyOnDrop(Some(cancelled));
            started.send(()).expect("test is waiting for startup");
            std::future::pending::<ClientResult<()>>().await
        }));
        tokio::time::timeout(Duration::from_secs(2), ready)
            .await
            .unwrap()
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), dropped)
            .await
            .unwrap()
            .unwrap();
    }
    #[tokio::test]
    async fn committed_writes_finish_when_the_view_stops_waiting() {
        let (started, ready) = oneshot::channel();
        let (resume, proceed) = oneshot::channel();
        let (completed, done) = oneshot::channel();
        let waiting = commit(async move {
            started.send(()).unwrap();
            proceed.await.unwrap();
            completed.send(()).unwrap();
            Ok(())
        });
        drop(waiting);
        tokio::time::timeout(Duration::from_secs(2), ready)
            .await
            .unwrap()
            .unwrap();
        resume.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), done)
            .await
            .unwrap()
            .unwrap();
    }
}
