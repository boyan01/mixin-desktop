//! Application-owned process state, independent of the official runner's main.
use std::{cell::Cell, rc::Rc, sync::OnceLock};

use nsh_core::{element::desktop, prelude::*};
use tokio::sync::mpsc;

use crate::{runtime, shell};

static QUIT: OnceLock<mpsc::UnboundedSender<()>> = OnceLock::new();

fn signal_quit() {
    if let Some(sender) = QUIT.get() {
        let _ = sender.send(());
    }
}

thread_local! {
    static INSTANCE: std::cell::RefCell<Option<shell::instance::Instance>> = const { std::cell::RefCell::new(None) };
}
static PREPARED: OnceLock<Result<bool, String>> = OnceLock::new();

/// The process host is created on AppKit's thread, before business services open.
pub(crate) fn prepare() {
    PREPARED.get_or_init(|| {
        let directory = mixin_desktop_api::data_directory().map_err(|e| e.to_string())?;
        match shell::prepare_launch(&directory, std::env::args().skip(1), None)
            .map_err(|e| e.to_string())?
        {
            shell::Launch::Primary(instance) => {
                mixin_desktop_api::init_logging(
                    "Mixin".into(),
                    env!("CARGO_PKG_VERSION").into(),
                    "2".into(),
                )
                .map_err(|error| error.to_string())?;
                INSTANCE.with(|slot| slot.replace(Some(instance)));
                Ok(true)
            }
            shell::Launch::Forwarded => Ok(false),
        }
    });
}

pub(crate) fn release() {
    INSTANCE.with(|slot| {
        slot.replace(None);
    });
}

pub(crate) fn mount() -> bool {
    match PREPARED.get() {
        Some(Ok(true)) => {}
        other => {
            if let Some(Err(error)) = other {
                eprintln!("Mixin startup failed: {error}");
            }
            desktop::quit();
            return false;
        }
    }
    let quitting = Rc::new(Cell::new(false));
    let request = {
        let quitting = quitting.clone();
        move || {
            if quitting.replace(true) {
                return;
            }
            spawn(async {
                // Drain on the business runtime before requesting final engine
                // disposal. The engine's 200 ms quit budget is not used here.
                let _ = runtime::run(async {
                    runtime::drain_shutdown().await;
                    Ok(())
                })
                .await;
                log::info!("Application shutdown drained");
                log::logger().flush();
                desktop::quit();
            });
        }
    };
    desktop::on_quit_request(request.clone());
    let (sender, receiver) = mpsc::unbounded_channel();
    let _ = QUIT.set(sender);
    if let Ok(signal) = runtime::quit_on_signal(signal_quit) {
        let signal = stored(Some(signal));
        cleanup(move || {
            signal.replace(None);
        });
    }
    spawn(async move {
        let mut receiver = Box::pin(runtime::observe(futures::stream::unfold(
            receiver,
            |mut receiver| async { receiver.recv().await.map(|event| (event, receiver)) },
        )));
        use futures::StreamExt as _;
        while receiver.next().await.is_some() {
            request();
        }
    });
    // Host disposal cancels UI work and starts account shutdown. The native
    // process host below owns the final bounded drain, outside this short hook.
    desktop::on_app_quit(|| {
        runtime::stop();
        async {}
    });
    desktop::current_window().on_close_press(|_| desktop::current_window().hide());
    true
}

/// The process host is instantiated only by App, never by Preview or discovery.
pub(crate) fn surface(render: impl Fn() -> AnyElement + 'static) -> impl IntoElement {
    let ready = signal(false);
    view()
        .flex_1()
        .child(
            native("MixinProcess")
                .w_0()
                .h_0()
                .on_native_event(move |event: &NativeEvent| {
                    if event.name.as_ref() == "ready" {
                        ready.set(true);
                    }
                }),
        )
        .child(dynamic(move || {
            if ready.get() && mount() {
                render()
            } else {
                view().into_element()
            }
        }))
}

pub(crate) fn register(components: &nsh_platform_macos::Components) {
    use nsh_platform_macos::{NativeComponent, objc2_app_kit::NSView};
    use objc2::{MainThreadMarker, rc::Retained};
    struct Host {
        view: Retained<NSView>,
    }
    impl NativeComponent for Host {
        fn view(&self) -> Retained<NSView> {
            self.view.clone()
        }
        fn set_prop(&mut self, _: &str, _: Option<&serde_json::Value>) {}
    }
    impl Drop for Host {
        fn drop(&mut self) {
            if matches!(PREPARED.get(), Some(Ok(true))) {
                std::thread::scope(|scope| {
                    scope
                        .spawn(runtime::finish_shutdown)
                        .join()
                        .expect("shutdown worker");
                });
                release();
            }
        }
    }
    components.register(
        "MixinProcess",
        Rc::new(|context| {
            prepare();
            let view = NSView::new(MainThreadMarker::new().expect("AppKit thread"));
            context.emit("ready", serde_json::Value::Null);
            Box::new(Host { view })
        }),
    );
}
