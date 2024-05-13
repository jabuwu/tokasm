#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::{atomic::AtomicU64, Arc, Once};

    use tokio::sync::Notify;

    pub(crate) struct Context {
        pub(crate) runtime: tokio::runtime::Runtime,
        pub(crate) task_count: AtomicU64,
        pub(crate) shutdown: Arc<Notify>,
    }

    impl Context {
        pub(crate) fn singleton() -> &'static Context {
            static START: Once = Once::new();
            static mut INSTANCE: Option<Context> = None;
            START.call_once(|| unsafe {
                INSTANCE = Some(Context::new());
            });
            unsafe { &INSTANCE.as_ref().unwrap() }
        }

        fn new() -> Self {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let shutdown = Arc::new(Notify::new());
            Self {
                runtime,
                task_count: AtomicU64::new(1), // main thread counts as a task
                shutdown,
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
