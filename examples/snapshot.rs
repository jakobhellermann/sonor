use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), sonos::Error> {
    let transport_uri = "http://192.168.2.91:1234/google_tts_proxy/aHR0cHM6Ly90cmFuc2xhdGUuZ29vZ2xlLmNvbS90cmFuc2xhdGVfdHRzP2llPVVURi04JnE9dGVzdCZ0bD1lbiZ0az02ODU5ODQuODQ5OTU1JmNsaWVudD13ZWJhcHA=/test.mp3";
    let speaker = sonos::find("jakob", Duration::from_secs(3)).await?.unwrap();

    let snapshot = speaker.snapshot().await?;
    println!("{:#?}", snapshot);

    speaker.set_volume(10).await?;
    speaker.set_transport_uri(transport_uri, "").await?;
    speaker.play().await?;
    tokio::time::delay_for(Duration::from_secs(3)).await;

    speaker.apply(snapshot).await?;

    Ok(())
}
