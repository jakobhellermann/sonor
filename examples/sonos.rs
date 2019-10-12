use futures::prelude::*;
use sonos::Speaker;
use std::time::Duration;

fn main() {
    if let Err(e) = async_std::task::block_on(sonos()) {
        eprintln!("{}", e);
    }
}

async fn sonos() -> Result<(), upnp::Error> {
    let speaker = my_speaker().await?;
    if let Some(speaker) = speaker {
        print_speaker_info(speaker).await?;
    }

    Ok(())
}

#[allow(unused)]
async fn my_speaker() -> Result<Option<Speaker>, upnp::Error> {
    Speaker::from_ip([192, 168, 2, 29].into())
        .await
        .map(|x| Some(x.expect("ip is sonos device")))
}
#[allow(unused)]
async fn find_speaker() -> Result<Option<Speaker>, upnp::Error> {
    let stream = sonos::discover(Duration::from_secs(3)).await?;
    pin_utils::pin_mut!(stream);

    stream.next().await.transpose()
}

async fn print_speaker_info(speaker: Speaker) -> Result<(), sonos::upnp::Error> {
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
        let coordinator = speakers
            .iter()
            .find(|s| s.uuid().eq_ignore_ascii_case(&coordinator))
            .expect("no coordinator for group");
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
