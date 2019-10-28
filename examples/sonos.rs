use async_std::task;
use sonos::Speaker;
use std::time::Duration;

type Result<T> = std::result::Result<T, sonos::Error>;

fn main() {
    if let Err(e) = task::block_on(print_speaker_info()) {
        eprintln!("{}", e);
    }
}

async fn print_speaker_info() -> Result<()> {
    let speaker = sonos::find("jakob", Duration::from_secs(3))
        .await?
        .expect("speaker exists");

    general(&speaker).await?;
    currently_playing(&speaker).await?;
    equalizer(&speaker).await?;
    group_state(&speaker).await?;

    Ok(())
}

async fn general(speaker: &Speaker) -> Result<()> {
    println!("- Name: {}", speaker.name().await?);
    println!();
    Ok(())
}

async fn currently_playing(speaker: &Speaker) -> Result<()> {
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

    Ok(())
}

async fn equalizer(_speaker: &Speaker) -> Result<()> {
    Ok(())
}

async fn group_state(speaker: &Speaker) -> Result<()> {
    println!("Groups: ");
    let groups = speaker.zone_group_state().await?;
    for (coordinator, speakers) in groups {
        let coordinator = speakers
            .iter()
            .find(|s| s.uuid().eq_ignore_ascii_case(&coordinator))
            .expect("no coordinator for group");

        println!(" - {} : {}", coordinator.name(), coordinator.uuid());
        for speaker in speakers {
            println!("   - {} : {}", speaker.name(), speaker.uuid());
        }
    }
    Ok(())
}
