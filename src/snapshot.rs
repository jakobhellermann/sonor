use crate::{track::TrackInfo, Result, Speaker};
use futures_util::future::{try_join, try_join4};

/// A Snapshot of the state the speaker is in right now.
/// Useful for announcing some clip at a lower volume, then later resume where you left of.
#[derive(Debug, Default)]
pub struct Snapshot {
    volume: Option<u16>,
    is_playing: Option<bool>,
    track_info: Option<TrackInfo>,

    transport_uri: Option<String>,
}

impl Snapshot {
    /// Sets the volume of the snapshot
    pub fn volume(&mut self, volume: u16) -> &mut Self {
        self.volume = Some(volume);
        self
    }

    /// Sets whether the speaker is playing
    pub fn is_playing(&mut self, is_playing: bool) -> &mut Self {
        self.is_playing = Some(is_playing);
        self
    }

    /// Specifies the current track info
    pub fn track_info(&mut self, track_info: TrackInfo) -> &mut Self {
        self.track_info = Some(track_info);
        self
    }

    /// Specifies the current track info
    pub fn transport_uri(&mut self, transport_uri: impl Into<String>) -> &mut Self {
        self.transport_uri = Some(transport_uri.into());
        self
    }

    pub(crate) async fn from_speaker(speaker: &Speaker) -> Result<Self> {
        let (volume, track_info, is_playing, transport_uri) = try_join4(
            speaker.volume(),
            speaker.track(),
            speaker.is_playing(),
            speaker.transport_uri(),
        )
        .await?;

        Ok(Self {
            volume: Some(volume),
            track_info,
            is_playing: Some(is_playing),
            transport_uri,
        })
    }

    pub(crate) async fn apply(&self, speaker: &Speaker) -> Result<()> {
        if let Some(volume) = self.volume {
            speaker.set_volume(volume).await?;
        }

        match &self.transport_uri {
            Some(uri) if uri.starts_with("x-sonos-vli") => {
                log::warn!("unsupported transport uri: 'x-sonos-vli:...'")
            }
            Some(uri) => speaker.set_transport_uri(uri, "").await?,
            None => {}
        }

        if let Some(track_info) = &self.track_info {
            try_join(
                speaker.seek_track(track_info.track_no()),
                speaker.skip_to(track_info.elapsed()),
            )
            .await?;
        }

        match self.is_playing {
            Some(false) => speaker.pause().await?,
            Some(true) => speaker.play().await?,
            None => {}
        }

        Ok(())
    }
}
