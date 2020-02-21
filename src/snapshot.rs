use crate::{track::TrackInfo, Result, Speaker};
use futures_util::future::{try_join, try_join4};

/// A Snapshot of the state the speaker is in right now.
/// Useful for announcing some clip at a lower volume, then later resume where you left of.
#[derive(Debug)]
pub struct Snapshot {
    volume: u16,
    track_info: Option<TrackInfo>,
    is_playing: bool,

    transport_uri: String,
}

impl Snapshot {
    pub(crate) async fn new(speaker: &Speaker) -> Result<Self> {
        let (volume, track_info, is_playing, transport_uri) = try_join4(
            speaker.volume(),
            speaker.track(),
            speaker.is_playing(),
            speaker.transport_uri(),
        )
        .await?;

        Ok(Self {
            volume,
            track_info,
            is_playing,
            transport_uri,
        })
    }

    pub(crate) async fn apply(&self, speaker: &Speaker) -> Result<()> {
        let (_, new_transport_uri) =
            try_join(speaker.set_volume(self.volume), speaker.transport_uri()).await?;

        if self.transport_uri == new_transport_uri {
            speaker.set_transport_uri(&self.transport_uri, "").await?;
        }

        if let Some(track_info) = &self.track_info {
            speaker.seek_track(track_info.track_no()).await?;
            speaker.skip_to(track_info.elapsed()).await?;
        }

        if self.is_playing {
            speaker.play().await?;
        } else {
            speaker.pause().await?;
        }

        Ok(())
    }
}
