use roxmltree::Node;
use std::fmt;

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

#[derive(Debug, Eq, Hash)]
pub struct SpeakerInfo {
    room_name: String,
    uuid: String,
    location: String,
}
impl std::cmp::PartialEq for SpeakerInfo {
    fn eq(&self, other: &Self) -> bool {
        self.uuid().eq_ignore_ascii_case(other.uuid())
    }
}

impl SpeakerInfo {
    pub(crate) fn from_xml(node: Node) -> Result<Self, upnp::Error> {
        let mut uuid = None;
        let mut room_name = None;
        let mut location = None;

        for attr in node.attributes() {
            match attr.name().to_lowercase().as_str() {
                "uuid" => uuid = Some(attr.value()),
                "location" => location = Some(attr.value()),
                "zonename" => room_name = Some(attr.value()),
                _ => (),
            }
        }

        Ok(Self {
            room_name: room_name
                .ok_or(upnp::Error::XMLMissingElement(
                    "RoomName".to_string(),
                    "ZoneGroupMember".to_string(),
                ))?
                .to_string(),
            uuid: uuid
                .ok_or(upnp::Error::XMLMissingElement(
                    "RoomName".to_string(),
                    "ZoneGroupMember".to_string(),
                ))?
                .to_string(),
            location: location
                .ok_or(upnp::Error::XMLMissingElement(
                    "RoomName".to_string(),
                    "ZoneGroupMember".to_string(),
                ))?
                .to_string(),
        })
    }

    pub fn room_name(&self) -> &String {
        &self.room_name
    }
    pub fn uuid(&self) -> &String {
        &self.uuid
    }
    pub fn location(&self) -> &String {
        &self.location
    }
}
