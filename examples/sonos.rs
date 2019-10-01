use sonos::Speaker;

fn main() {
    if let Err(e) = async_std::task::block_on(sonos()) {
        eprintln!("{}", e);
    }
}

async fn sonos() -> Result<(), sonos::upnp::Error> {
    let speaker = Speaker::from_ip([192, 168, 2, 49].into())
        .await?
        .expect("ip is sonos device");

    let name = speaker.name().await?;
    println!("- Name: {}", name);

    let track_info = speaker.track().await?;
    if let Some(track_info) = track_info {
        println!("- Currently playing '{}'", track_info.track());
    } else {
        println!("- No track currently playing");
    }

    let queue = speaker.queue().await?;
    println!(
        "- {} track{}in queue",
        queue.len(),
        if queue.len() == 1 { " " } else { "s " }
    );
    for track in queue.iter().skip(1).take(5) {
        println!("  - {}", track);
    }
    println!("  - ...\n");

    println!("Groups: ");
    let groups = speaker.group_topology().await?;
    for (coordinator, speakers) in groups {
        let coordinator = speakers.iter().find(|s| s.uuid() == &coordinator).unwrap();
        println!(
            " - {}:{} @ {}:",
            coordinator.room_name(),
            coordinator.uuid(),
            coordinator.location()
        );
        for speaker in speakers {
            println!("   - {}", speaker.room_name());
        }
    }

    Ok(())
}
