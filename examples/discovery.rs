use futures::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), sonor::Error> {
    let mut devices = sonor::discover(Duration::from_secs(5)).await?;

    while let Some(device) = devices.try_next().await? {
        let name = device.name().await?;
        println!("- {}", name);
    }

    Ok(())
}
