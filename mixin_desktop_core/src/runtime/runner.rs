//! Account-loop execution and completion independent of account business state.
use super::{run_account, AccountRunContext};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::watch;

pub(super) struct Runner {
    running: Arc<AtomicBool>,
    done: watch::Receiver<Option<Exit>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Exit {
    Stopped,
    Unauthorized,
}
struct Completion {
    running: Arc<AtomicBool>,
    done: watch::Sender<Option<Exit>>,
    exit: Exit,
}
impl Drop for Completion {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        self.done.send_replace(Some(self.exit));
    }
}
impl Runner {
    pub fn start(name: String, context: AccountRunContext) -> anyhow::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let (done, receiver) = watch::channel(None);
        let completion = Completion {
            running: running.clone(),
            done,
            exit: Exit::Stopped,
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::Builder::new().name(name).spawn(move || {
            let mut completion = completion;
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => completion.exit = runtime.block_on(run_account(context)),
                Err(error) => {
                    let _ = context.ready_sender.send(Err(error.to_string()));
                }
            }
        })?;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = name;
            wasm_bindgen_futures::spawn_local(async move {
                let mut completion = completion;
                completion.exit = run_account(context).await;
            });
        }
        Ok(Self {
            running,
            done: receiver,
        })
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
    pub fn completion(&self) -> watch::Receiver<Option<Exit>> {
        self.done.clone()
    }
    pub async fn wait(&self) {
        let mut done = self.done.clone();
        while done.borrow_and_update().is_none() {
            if done.changed().await.is_err() {
                break;
            }
        }
    }

    #[cfg(test)]
    pub(super) fn observing_shutdown(mut shutdown: watch::Receiver<bool>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let (done, receiver) = watch::channel(None);
        let completion = Completion {
            running: running.clone(),
            done,
            exit: Exit::Stopped,
        };
        tokio::spawn(async move {
            let _completion = completion;
            while !*shutdown.borrow_and_update() {
                if shutdown.changed().await.is_err() {
                    return;
                }
            }
        });
        Self {
            running,
            done: receiver,
        }
    }
}
