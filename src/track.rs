use crate::{utils, Result};
use roxmltree::Node;

/// A [Track](struct.Track.html) with some metadata like the track number, its duration and the
/// elapsed time.
#[derive(Debug)]
pub struct TrackInfo {
    track: Track,
    metadata: String,
    track_no: u32,
    duration: u32,
    elapsed: u32,
}

#[allow(missing_docs)]
impl TrackInfo {
    pub(crate) fn new(
        track: Track,
        metadata: String,
        track_no: u32,
        duration: u32,
        elapsed: u32,
    ) -> Self {
        Self {
            track,
            metadata,
            track_no,
            duration,
            elapsed,
        }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }
    pub fn metadata(&self) -> &str {
        &self.metadata
    }
    pub fn track_no(&self) -> u32 {
        self.track_no
    }
    pub fn duration(&self) -> u32 {
        self.duration
    }
    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }
}

/// The track struct contains information about the music in UPnP music players.
/// It always has a title and an URI, but sometimes there is a creator, album or duration specified
/// too.
#[derive(Debug)]
pub struct Track {
    title: String,
    creator: Option<String>,
    album: Option<String>,
    duration: Option<u32>,
    uri: String,
}

#[allow(missing_docs)]
impl Track {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }
    pub fn album(&self) -> Option<&str> {
        self.album.as_deref()
    }
    pub fn duration(&self) -> Option<u32> {
        self.duration
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.title)?;
        if let Some(creator) = &self.creator {
            write!(f, " - {}", creator)?;
        }
        if let Some(album) = &self.album {
            write!(f, " ({})", album)?;
        }
        Ok(())
    }
}

impl Track {
    pub(crate) fn from_xml(node: Node<'_, '_>) -> Result<Self> {
        let mut title = None;
        let mut creator = None;
        let mut album = None;
        let mut res = None;

        for child in node.children() {
            match child.tag_name().name() {
                "title" => title = Some(child.text().unwrap_or_default().to_string()),
                "creator" => creator = Some(child.text().unwrap_or_default().to_string()),
                "album" => album = Some(child.text().unwrap_or_default().to_string()),
                "res" => res = Some(child),
                _ => (),
            }
        }

        let title = title.ok_or_else(|| {
            rupnp::Error::XmlMissingElement(node.tag_name().name().to_string(), "title".to_string())
        })?;
        let res = res.ok_or_else(|| {
            rupnp::Error::XmlMissingElement(node.tag_name().name().to_string(), "res".to_string())
        })?;
        let duration = res
            .attributes()
            .find(|a| a.name().eq_ignore_ascii_case("duration"))
            .map(|a| utils::seconds_from_str(a.value()))
            .transpose()?;

        let uri = res.text().unwrap_or_default().to_string();

        Ok(Self {
            title,
            creator,
            album,
            duration,
            uri,
        })
    }
}
