#![feature(async_await)]
#![recursion_limit = "128"]

use std::time::Duration;

#[runtime::main(runtime_tokio::Tokio)]
async fn main() -> Result<(), failure::Error> {
    for player in sonos::discover(Duration::from_secs(1)).await? {
        println!("{}", player.get_name().await?);
    }

    Ok(())
}
