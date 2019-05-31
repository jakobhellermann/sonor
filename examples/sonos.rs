#![feature(async_await, await_macro)]
#![recursion_limit = "128"]

use sonos::Speaker;

#[runtime::main(runtime_tokio::Tokio)]
async fn main() -> Result<(), failure::Error> {
    let player = Speaker::from_ip([192, 168, 2, 49].into())
        .await?
        .expect("ip is sonos device");

    let name = player.get_name().await?;
    println!("- Name: {}", name);

    let track_info = player.track().await?;
    if let Some(track_info) = track_info {
        println!("- Currently playing '{}'", track_info.track());
    } else {
        println!("- No track currently playing");
    }

    let queue = player.get_queue().await?;
    println!(
        "- {} track{}in queue",
        queue.len(),
        if queue.len() == 1 { " " } else { "s " }
    );
    for track in queue.iter().skip(1).take(5) {
        println!("  - {}", track);
    }
    println!("  - ...");

    Ok(())
}
