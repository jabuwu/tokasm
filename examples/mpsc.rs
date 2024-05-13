use std::time::Duration;

use shadow_clone::shadow_clone;
use tokasm::sync::mpsc;
use tracing::{info, Level};

#[tokasm::main]
async fn main() {
    unilog::init(Level::INFO, "");

    let (sender, mut receiver) = mpsc::channel::<String>(100);
    {
        shadow_clone!(sender);
        tokasm::spawn(async move {
            sender.send("Hello".to_owned()).await.unwrap();
            tokasm::time::sleep(Duration::from_millis(1000)).await;
            sender.send("world!".to_owned()).await.unwrap();
        });
    }
    tokasm::spawn(async move {
        tokasm::time::sleep(Duration::from_millis(500)).await;
        sender.send("wherever you are in the".to_owned()).await.unwrap();
    });
    tokasm::spawn(async move {
        while let Some(message) = receiver.recv().await {
            info!("{}", message);
        }
    });
}
