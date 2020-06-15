use sonor::Speaker;
use std::time::Duration;

type Result<T, E = sonor::Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<()> {
    let roomname = std::env::args()
        .nth(1)
        .expect("expected room name as first argument");

    let speaker = sonor::find(&roomname, Duration::from_secs(3))
        .await?
        .unwrap_or_else(|| panic!("speaker '{}' doesn't exist", roomname));

    general(&speaker).await?;
    currently_playing(&speaker).await?;
    equalizer(&speaker).await?;
    group_state(&speaker).await?;

    Ok(())
}

async fn general(speaker: &Speaker) -> Result<()> {
    println!("Name: {}", speaker.name().await?);
    Ok(())
}

async fn currently_playing(speaker: &Speaker) -> Result<()> {
    println!();

    let track_info = speaker.track().await?;
    if let Some(track_info) = track_info {
        let duration = fmt_duration(track_info.duration());
        let elapsed = fmt_duration(track_info.elapsed());
        println!(
            "Currently playing: '{}' [{}/{}]",
            track_info.track(),
            elapsed,
            duration
        );
    } else {
        println!("No track are currently playing...");
        return Ok(());
    }

    let queue = &speaker.queue().await?;

    match queue.len() {
        0 => println!("There are no tracks coming after that."),
        1 => println!("1 track in queue:"),
        n => println!("{} tracks in queue:", n),
    }

    for track in queue.iter().take(5) {
        println!(" - {}", track);
    }
    if queue.len() > 5 {
        println!(" - ...");
    }

    Ok(())
}

async fn equalizer(_speaker: &Speaker) -> Result<()> {
    Ok(())
}

async fn group_state(speaker: &Speaker) -> Result<()> {
    let groups: Vec<_> = speaker
        .zone_group_state()
        .await?
        .into_iter()
        .filter(|(_, speakers)| speakers.len() > 1)
        .collect();

    if groups.is_empty() {
        return Ok(());
    }

    println!();
    println!("Groups: ");
    for (coordinator, speakers) in groups {
        let coordinator = speakers
            .iter()
            .find(|s| s.uuid().eq_ignore_ascii_case(&coordinator))
            .expect("no coordinator for group");

        println!(" - {}", coordinator.name());
        for speaker in speakers {
            println!("   - {} : {}", speaker.name(), speaker.uuid());
        }
    }
    Ok(())
}

fn fmt_duration(secs: u32) -> String {
    return format!("{:02}:{:02}", secs / 60, secs % 60);
}
