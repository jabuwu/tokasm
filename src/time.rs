#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::time::Duration;

    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::time::Duration;

    use js_sys::Promise;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::window;

    pub async fn sleep(duration: Duration) {
        let window = window().unwrap();
        JsFuture::from(Promise::new(&mut |resolve, _reject| {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve,
                    duration.as_millis() as i32,
                )
                .unwrap();
        }))
        .await
        .unwrap();
    }
}
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
