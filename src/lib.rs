pub use tokasm_macros::main;

pub mod time;
pub mod sync;

#[cfg(not(target_arch = "wasm32"))]
mod context;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use context::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{future::Future, sync::atomic::Ordering};

    use crate::Context;

    pub fn spawn<F>(future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let context = Context::singleton();
        context.task_count.fetch_add(1, Ordering::SeqCst);
        context.runtime.spawn(async move {
            future.await;
            if context.task_count.fetch_sub(1, Ordering::SeqCst) == 1 {
                context.shutdown.notify_one();
            }
        });
    }

    pub fn wait_until_finished() {
        let context = Context::singleton();
        context.runtime.block_on(async move {
            tokio::task::yield_now().await;
            // decrement the task count (we count the main thread as a task)
            // if there were other tasks running, wait for them
            let task_count = context.task_count.fetch_sub(1, Ordering::SeqCst);
            if task_count > 1 {
                context.shutdown.notified().await;
            }
        });
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::future::Future;

    use wasm_bindgen_futures::spawn_local;

    pub fn spawn<F>(future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        spawn_local(async move {
            future.await;
        });
    }
}
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
