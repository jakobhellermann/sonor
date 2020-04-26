use futures::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), sonos::Error> {
    let devices = sonos::discover(Duration::from_secs(2)).await?;
    futures::pin_mut!(devices);

    while let Some(device) = devices.try_next().await? {
        let name = device.name().await?;
        println!("- {}", name);
    }

    Ok(())
}
