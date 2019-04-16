#![feature(async_await, await_macro, futures_api)]
#![allow(unused)]

use sonos::Speaker;
use upnp::Device;

use futures::prelude::*;

use std::net::Ipv4Addr;

fn main() {
    tokio::run(
        async_main()
            .map_err(|e| eprintln!("{}", e))
            .boxed()
            .compat(),
    );
}

async fn async_main() -> Result<(), failure::Error> {
    let player: Speaker = await!(Speaker::from_ip(Ipv4Addr::new(192, 168, 2, 49)))?.unwrap();

    let name = await!(player.get_name())?;
    println!("- Name: {}", name);

    let track_info = await!(player.track())?;
    if let Some(track_info) = track_info {
        println!("- Currently playing '{}'", track_info.track());
    } else {
        println!("- No track currently playing");
    }

    let queue = await!(player.get_queue())?;
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
