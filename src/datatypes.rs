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
/*
#[derive(Debug)]
pub struct SpeakerInfo {
    room_name: String,
    uuid: String,
    location: String,
    coordinator: bool,
}
impl SpeakerInfo {
    /*pub(crate) fn from_xml(mut zone: Element, coordinator_uuid: &String) -> Option<Self> {
        let room_name = zone.attributes.remove("ZoneName")?;
        let uuid = zone.attributes.remove("UUID")?;
        let location = zone.attributes.remove("Location")?;
        Some(Self {
            coordinator: coordinator_uuid == &uuid,
            room_name,
            uuid,
            location,
        })
    }*/

    pub fn room_name(&self) -> &String {
        &self.room_name
    }
    pub fn uuid(&self) -> &String {
        &self.uuid
    }
    pub fn location(&self) -> &String {
        &self.location
    }
    pub fn coordinator(&self) -> bool {
        self.coordinator
    }
}*/
