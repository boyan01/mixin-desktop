//! Keep browser database handles and account tasks on one dedicated Worker.

#[cfg(not(target_arch = "wasm32"))]
flutter_rust_bridge::for_generated::lazy_static! {
    pub static ref FLUTTER_RUST_BRIDGE_HANDLER:
        flutter_rust_bridge::DefaultHandler<flutter_rust_bridge::SimpleThreadPool> =
        flutter_rust_bridge::DefaultHandler::new_simple(Default::default());
}

#[cfg(target_arch = "wasm32")]
flutter_rust_bridge::for_generated::lazy_static! {
    pub static ref FLUTTER_RUST_BRIDGE_HANDLER:
        flutter_rust_bridge::handler::SimpleHandler<browser::Executor, flutter_rust_bridge::handler::NoOpErrorListener> =
        flutter_rust_bridge::handler::SimpleHandler::new(browser::Executor, flutter_rust_bridge::handler::NoOpErrorListener);
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use flutter_rust_bridge::{
        for_generated::{MessagePort, TransferClosure},
        handler::{
            Executor as ExecutorTrait, NoOpErrorListener, SimpleExecutor, TaskContext, TaskInfo,
            TaskRetFutTrait,
        },
        BaseCodec, BaseThreadPool, SimpleAsyncRuntime,
    };
    use std::{cell::RefCell, future::Future};
    use wasm_bindgen::{prelude::*, JsCast};

    #[wasm_bindgen(module = "/web/worker.js")]
    extern "C" {
        #[wasm_bindgen(catch, js_name = createCoreWorker)]
        fn create_core_worker(module: JsValue, memory: JsValue) -> Result<Worker, JsValue>;
    }
    use web_sys::Worker;

    thread_local! {
        static WORKER: RefCell<Option<Worker>> = const { RefCell::new(None) };
    }

    // FRB's pool may spawn another Worker while one call is pending. Pin all
    // calls to the first Worker instead: OPFS and JS handle tables are local.
    pub struct SingleWorker;
    impl BaseThreadPool for SingleWorker {
        fn execute(&self, closure: TransferClosure<JsValue>) {
            WORKER.with(|slot| {
                let mut slot = slot.borrow_mut();
                if let Some(worker) = slot.as_ref() {
                    closure.apply(worker).expect("send call to core Worker");
                } else {
                    let worker = create_core_worker(wasm_bindgen::module(), wasm_bindgen::memory())
                        .expect("start core Worker");
                    closure.apply(&worker).expect("send first core call");
                    *slot = Some(worker);
                }
            });
        }
    }

    fn local() -> SimpleExecutor<NoOpErrorListener, SingleWorker, SimpleAsyncRuntime> {
        SimpleExecutor::new(NoOpErrorListener, SingleWorker, SimpleAsyncRuntime)
    }

    pub struct Executor;
    impl ExecutorTrait for Executor {
        fn execute_normal<C, F>(&self, info: TaskInfo, task: F)
        where
            C: BaseCodec,
            F: FnOnce(TaskContext) -> Result<C::Message, C::Message> + Send + 'static,
        {
            local().execute_normal::<C, F>(info, task);
        }

        fn execute_sync<C, F>(&self, info: TaskInfo, task: F) -> C::Message
        where
            C: BaseCodec,
            F: FnOnce() -> Result<C::Message, C::Message>,
        {
            // Synchronous bridge getters only clone handles or read snapshots;
            // database and network operations must remain asynchronous.
            local().execute_sync::<C, F>(info, task)
        }

        fn execute_async<C, F, Fut>(&self, info: TaskInfo, task: F)
        where
            C: BaseCodec,
            F: FnOnce(TaskContext) -> Fut + Send + 'static,
            Fut: Future<Output = Result<C::Message, C::Message>> + TaskRetFutTrait,
        {
            let TaskInfo {
                port,
                debug_name,
                mode,
            } = info;
            let port = port.expect("async bridge call has a port");
            let (raw, transfer) = if let Some(channel) = port.dyn_ref::<web_sys::BroadcastChannel>()
            {
                (JsValue::from(channel.name()), vec![])
            } else {
                let raw: JsValue = port.into();
                (raw.clone(), vec![raw])
            };
            SingleWorker.execute(TransferClosure::new(vec![raw], transfer, move |values| {
                let port = if let Some(name) = values[0].as_string() {
                    MessagePort::broadcast(&name)
                } else {
                    values[0].clone().unchecked_into()
                };
                local().execute_async::<C, F, Fut>(
                    TaskInfo {
                        port: Some(port),
                        debug_name,
                        mode,
                    },
                    task,
                );
            }));
        }
    }
}
