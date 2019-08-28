use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), sonos::upnp::Error> {
    for player in sonos::discover(Duration::from_secs(1)).await? {
        println!("{}", player.get_name().await?);
    }

    Ok(())
}
