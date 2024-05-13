use std::{sync::Arc, time::Duration};

use tokasm::sync::RwLock;
use tracing::{info, Level};
use shadow_clone::shadow_clone;

#[tokasm::main]
async fn main() {
    unilog::init(Level::INFO, "");

    let lock = Arc::new(RwLock::new(0));
    {
        shadow_clone!(lock);
        tokasm::spawn(async move {
            loop {
                *lock.write().await += 1;
                tokasm::time::sleep(Duration::from_millis(1000)).await;
            }
        });
    }
    {
        shadow_clone!(lock);
        tokasm::spawn(async move {
            loop {
                info!("{}", *lock.read().await);
                tokasm::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }
}
