use futures::prelude::*;
use std::time::Duration;

fn main() {
    if let Err(e) = async_std::task::block_on(discover()) {
        eprintln!("{}", e);
    }
}

async fn discover() -> Result<(), sonos::upnp::Error> {
    let mut devices = sonos::discover(Duration::from_secs(2)).await?;
    while let Some(device) = devices.next().await {
        let device = device?;
        let name = device.name().await?;
        println!("- {}", name);
    }

    Ok(())
}
