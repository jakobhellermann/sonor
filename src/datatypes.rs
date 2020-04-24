use crate::Result;
use roxmltree::Node;
use std::{
    cmp::PartialEq,
    fmt,
    hash::{Hash, Hasher},
};

/// This enum describes how Sonos repeats the current playlist.
#[derive(Debug)]
pub enum RepeatMode {
    /// The playlist doesn't get repeated.
    None,
    /// Only one song gets played on and on.
    One,
    /// The whole playlist is repeated.
    All,
}
impl std::default::Default for RepeatMode {
    fn default() -> Self {
        RepeatMode::None
    }
}

impl fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug)]
pub struct ParseRepeatModeError;
impl std::error::Error for ParseRepeatModeError {}
impl std::fmt::Display for ParseRepeatModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "provided string was not `NONE` or `ONE` or `ALL`".fmt(f)
    }
}

impl std::str::FromStr for RepeatMode {
    type Err = ParseRepeatModeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "none" => Ok(RepeatMode::None),
            "one" => Ok(RepeatMode::One),
            "all" => Ok(RepeatMode::All),
            _ => Err(ParseRepeatModeError),
        }
    }
}

/// A more lightweight representation of a speaker containing only the name, uuid and location.
/// It gets returned by the [zone_group_state](struct.Speaker.html#method.zone_group_state) function.
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

#[allow(missing_docs)]
impl SpeakerInfo {
    pub(crate) fn from_xml(node: Node<'_, '_>) -> Result<Self> {
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
                    rupnp::Error::XmlMissingElement(
                        "RoomName".to_string(),
                        "ZoneGroupMember".to_string(),
                    )
                })?
                .to_string(),
            uuid: uuid
                .ok_or_else(|| {
                    rupnp::Error::XmlMissingElement(
                        "UUID".to_string(),
                        "ZoneGroupMember".to_string(),
                    )
                })?
                .to_string(),
            location: location
                .ok_or_else(|| {
                    rupnp::Error::XmlMissingElement(
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
