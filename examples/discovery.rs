use futures::prelude::*;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), sonos::Error> {
    let devices = sonos::discover(Duration::from_secs(2)).await?;
    futures::pin_mut!(devices);

    while let Some(device) = devices.next().await {
        let device = device?;
        let name = device.name().await?;
        println!("- {}", name);
    }

    Ok(())
}
