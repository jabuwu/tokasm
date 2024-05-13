use std::time::Duration;

use tokasm::sync::oneshot;
use tracing::{info, Level};

#[tokasm::main]
async fn main() {
    unilog::init(Level::INFO, "");

    let (sender, receiver) = oneshot::channel::<String>();
    tokasm::spawn(async move {
        tokasm::time::sleep(Duration::from_millis(1000)).await;
        sender.send("Hello world!".to_owned()).unwrap();
    });
    tokasm::spawn(async move {
        info!("{}", receiver.await.unwrap());
    });
}
