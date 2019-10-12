use crate::utils;
use roxmltree::Node;
use std::time::Duration;

#[derive(Debug)]
pub struct TrackInfo {
    track: Track,
    track_no: u32,
    duration: Duration,
    elapsed: Duration,
}
impl TrackInfo {
    pub fn new(track: Track, track_no: u32, duration: Duration, played: Duration) -> Self {
        Self {
            track,
            track_no,
            duration,
            elapsed: played,
        }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }
    pub fn track_no(&self) -> u32 {
        self.track_no
    }
    pub fn duration(&self) -> &Duration {
        &self.duration
    }
    pub fn elapsed(&self) -> &Duration {
        &self.elapsed
    }
}

#[derive(Debug)]
pub struct Track {
    title: String,
    creator: Option<String>,
    album: Option<String>,
    duration: Option<Duration>,
    uri: String,
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
    pub(crate) fn from_xml(node: Node) -> Result<Self, upnp::Error> {
        let mut title = None;
        let mut creator = None;
        let mut album = None;
        let mut res = None;

        for child in node.children() {
            match child.tag_name().name() {
                "title" => title = Some(utils::parse_node_text(child)?),
                "creator" => creator = Some(utils::parse_node_text(child)?),
                "album" => album = Some(utils::parse_node_text(child)?),
                "res" => res = Some(child),
                _ => (),
            }
        }

        let title = title.ok_or_else(|| {
            upnp::Error::XMLMissingElement(node.tag_name().name().to_string(), "title".to_string())
        })?;
        let res = res.ok_or_else(|| {
            upnp::Error::XMLMissingElement(node.tag_name().name().to_string(), "res".to_string())
        })?;
        let duration = res
            .attributes()
            .iter()
            .find(|a| a.name().eq_ignore_ascii_case("duration"))
            .map(|a| utils::duration_from_str(a.value()))
            .transpose()?;

        let uri = utils::parse_node_text(res)?;

        Ok(Self {
            title,
            creator,
            album,
            duration,
            uri,
        })
    }
}
