use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), sonos::Error> {
    let transport_uri = "http://192.168.2.91:8082/jakob/say_proxy/aHR0cHM6Ly90cmFuc2xhdGUuZ29vZ2xlLmNvbS90cmFuc2xhdGVfdHRzP2llPVVURi04JnE9a2VrJnRsPWRlJnRrPTY4OTU1MC43ODkwMDUmY2xpZW50PXdlYmFwcA==/kek.mp3";

    let speaker = sonos::find("jakob", Duration::from_secs(3)).await?.unwrap();

    let snapshot = speaker.snapshot().await?;

    speaker.set_volume(20).await?;
    speaker.set_transport_uri(transport_uri, "").await?;
    speaker.play().await?;

    speaker.apply(snapshot).await?;

    Ok(())
}
