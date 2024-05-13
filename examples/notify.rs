use std::{sync::Arc, time::Duration};

use tokasm::sync::Notify;
use tracing::{info, Level};
use shadow_clone::shadow_clone;

#[tokasm::main]
async fn main() {
    unilog::init(Level::INFO, "");

    let notify = Arc::new(Notify::new());
    {
        shadow_clone!(notify);
        tokasm::spawn(async move {
            loop {
                tokasm::time::sleep(Duration::from_millis(1000)).await;
                info!("Wake up!");
                notify.notify_waiters();
            }
        });
    }
    {
        shadow_clone!(notify);
        tokasm::spawn(async move {
            loop {
                notify.notified().await;
                info!("  I'm awake!");
            }
        });
    }
    {
        shadow_clone!(notify);
        tokasm::spawn(async move {
            loop {
                notify.notified().await;
                info!("  I'm awake!");
            }
        });
    }
}
