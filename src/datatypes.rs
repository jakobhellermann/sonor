use roxmltree::Node;
use std::cmp::PartialEq;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum RepeatMode {
    None,
    One,
    All,
}
impl fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug)]
pub struct ParseRepeatModeError;
impl std::error::Error for ParseRepeatModeError {}
impl std::fmt::Display for ParseRepeatModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        "provided string was not `NONE` or `ONE` or `ALL`".fmt(f)
    }
}

impl std::str::FromStr for RepeatMode {
    type Err = ParseRepeatModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "none" => Ok(RepeatMode::None),
            "one" => Ok(RepeatMode::One),
            "all" => Ok(RepeatMode::All),
            _ => Err(ParseRepeatModeError),
        }
    }
}

#[derive(Debug, Eq)]
pub struct SpeakerInfo {
    pub(crate) name: String,
    pub(crate) uuid: String,
    pub(crate) location: String,
}
impl PartialEq for SpeakerInfo {
    fn eq(&self, other: &Self) -> bool {
        self.uuid().eq_ignore_ascii_case(other.uuid())
    }
}
impl Hash for SpeakerInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl SpeakerInfo {
    pub(crate) fn from_xml(node: Node) -> Result<Self, upnp::Error> {
        let mut uuid = None;
        let mut name = None;
        let mut location = None;

        for attr in node.attributes() {
            match attr.name().to_lowercase().as_str() {
                "uuid" => uuid = Some(attr.value()),
                "location" => location = Some(attr.value()),
                "zonename" => name = Some(attr.value()),
                _ => (),
            }
        }

        Ok(Self {
            name: name
                .ok_or_else(|| {
                    upnp::Error::XMLMissingElement(
                        "RoomName".to_string(),
                        "ZoneGroupMember".to_string(),
                    )
                })?
                .to_string(),
            uuid: uuid
                .ok_or_else(|| {
                    upnp::Error::XMLMissingElement(
                        "UUID".to_string(),
                        "ZoneGroupMember".to_string(),
                    )
                })?
                .to_string(),
            location: location
                .ok_or_else(|| {
                    upnp::Error::XMLMissingElement(
                        "Location".to_string(),
                        "ZoneGroupMember".to_string(),
                    )
                })?
                .to_string(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
    pub fn location(&self) -> &str {
        &self.location
    }
}
