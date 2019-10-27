use crate::track::{Track, TrackInfo};
use crate::utils::{self, HashMapExt};
use crate::{args, Result};
use crate::{RepeatMode, SpeakerInfo};

use upnp::ssdp_client::{SearchTarget, URN};
use upnp::Device;

use futures::prelude::*;
use roxmltree::{Attribute, Document, Node};

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;

const SONOS_URN: URN = URN::device("schemas-upnp-org", "ZonePlayer", 1);

const AV_TRANSPORT: &URN = &URN::service("schemas-upnp-org", "AVTransport", 1);
const DEVICE_PROPERTIES: &URN = &URN::service("schemas-upnp-org", "DeviceProperties", 1);
const RENDERING_CONTROL: &URN = &URN::service("schemas-upnp-org", "RenderingControl", 1);
const ZONE_GROUP_TOPOLOGY: &URN = &URN::service("schemas-upnp-org", "ZoneGroupTopology", 1);
const QUEUE: &URN = &URN::service("schemas-sonos-com", "Queue", 1);

const DEFAULT_ARGS: &str = "<InstanceID>0</InstanceID>";

#[derive(Debug)]
/// A sonos speaker.
pub struct Speaker(upnp::Device);

/// discover sonos players on the network and stream their responses
pub async fn discover(timeout: Duration) -> Result<impl Stream<Item = Result<Speaker>>> {
    let stream = upnp::discover(&SearchTarget::URN(SONOS_URN), timeout)
        .await?
        .map_ok(Speaker::from_device)
        .map_ok(|device| device.expect("searched for sonos urn but got something else"));

    Ok(stream)
}

impl Speaker {
    /// create a speaker from an already found UPnP-Device
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type() == &SONOS_URN {
            Some(Self(device))
        } else {
            None
        }
    }

    /// create a speaker from an IPv4 address.
    /// returns `Ok(None)` when the device was found but isn't a sonos player.
    pub async fn from_ip(addr: Ipv4Addr) -> Result<Option<Self>> {
        let uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .expect("is always valid");

        Device::from_url(uri).await.map(Speaker::from_device)
    }

    pub async fn name(&self) -> Result<String> {
        self.action(DEVICE_PROPERTIES, "GetZoneAttributes", "")
            .await?
            .extract("CurrentZoneName")
    }

    // AV_TRANSPORT
    pub async fn stop(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "Stop", DEFAULT_ARGS)
            .await
            .map(drop)
    }
    pub async fn play(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "Play", args! { "InstanceID": 0, "Speed": 1 })
            .await
            .map(drop)
    }
    pub async fn pause(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "Pause", DEFAULT_ARGS)
            .await
            .map(drop)
    }
    pub async fn next(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "Next", DEFAULT_ARGS)
            .await
            .map(drop)
    }
    pub async fn previous(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "Previous", DEFAULT_ARGS)
            .await
            .map(drop)
    }

    pub async fn skip_to(&self, time: &Duration) -> Result<()> {
        let args =
            args! { "InstanceID": 0, "Unit": "REL_TIME", "Target": utils::duration_to_str(time)};
        self.action(AV_TRANSPORT, "Seek", args).await.map(drop)
    }
    pub async fn skip_by(&self, time: &Duration) -> Result<()> {
        let args =
            args! { "InstanceID": 0, "Unit": "TIME_DELTA", "Target": utils::duration_to_str(time)};
        self.action(AV_TRANSPORT, "Seek", args).await.map(drop)
    }
    pub async fn go_to_track(&self, track_no: u32) -> Result<()> {
        let args = args! { "InstanceID": 0, "Unit": "TRACK_NR", "Target": track_no + 1};
        self.action(AV_TRANSPORT, "Seek", args).await.map(drop)
    }

    async fn playback_mode(&self) -> Result<(RepeatMode, bool)> {
        let play_mode = self
            .action(AV_TRANSPORT, "GetTransportSettings", DEFAULT_ARGS)
            .await?
            .extract("PlayMode")?;

        match play_mode.to_uppercase().as_str() {
            "NORMAL" => Ok((RepeatMode::None, false)),
            "REPEAT_ALL" => Ok((RepeatMode::All, false)),
            "REPEAT_ONE" => Ok((RepeatMode::One, false)),
            "SHUFFLE_NOREPEAT" => Ok((RepeatMode::None, true)),
            "SHUFFLE" => Ok((RepeatMode::All, true)),
            "SHUFFLE_REPEAT_ONE" => Ok((RepeatMode::One, true)),
            _ => Err(upnp::Error::invalid_response(
                crate::datatypes::ParseRepeatModeError,
            )),
        }
    }
    pub async fn repeat_mode(&self) -> Result<RepeatMode> {
        self.playback_mode()
            .await
            .map(|(repeat_mode, _)| repeat_mode)
    }
    pub async fn shuffle(&self) -> Result<bool> {
        self.playback_mode().await.map(|(_, shuffle)| shuffle)
    }

    async fn set_playback_mode(&self, repeat_mode: RepeatMode, shuffle: bool) -> Result<()> {
        let playback_mode = match (repeat_mode, shuffle) {
            (RepeatMode::None, false) => "NORMAL",
            (RepeatMode::One, false) => "REPEAT_ONE",
            (RepeatMode::All, false) => "REPEAT_ALL",
            (RepeatMode::None, true) => "SHUFFLE_NOREPEAT",
            (RepeatMode::One, true) => "SHUFFLE_REPEAT_ONE",
            (RepeatMode::All, true) => "SHUFFLE",
        };
        self.action(
            AV_TRANSPORT,
            "SetPlayMode",
            args! { "InstanceID": 0, "NewPlayMode": playback_mode },
        )
        .await
        .map(drop)
    }
    pub async fn set_repeat_mode(&self, repeat_mode: RepeatMode) -> Result<()> {
        self.set_playback_mode(repeat_mode, self.shuffle().await?)
            .await
    }
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.set_playback_mode(self.repeat_mode().await?, shuffle)
            .await
    }

    pub async fn crossfade(&self) -> Result<bool> {
        self.action(AV_TRANSPORT, "GetCrossfadeMode", DEFAULT_ARGS)
            .await?
            .extract("CrossfadeMode")
            .and_then(utils::parse_bool)
    }
    pub async fn set_crossfade(&self, crossfade: bool) -> Result<()> {
        let args = args! { "InstanceID": 0, "CrossfadeMode": crossfade as u8 };
        self.action(AV_TRANSPORT, "SetCrossfadeMode", args)
            .await
            .map(drop)
    }

    pub async fn is_playing(&self) -> Result<bool> {
        self.action(AV_TRANSPORT, "GetTransportInfo", DEFAULT_ARGS)
            .await?
            .extract("CurrentTransportState")
            .map(|x| x.eq_ignore_ascii_case("playing"))
    }

    pub async fn track(&self) -> Result<Option<TrackInfo>> {
        let mut map = self
            .action(AV_TRANSPORT, "GetPositionInfo", DEFAULT_ARGS)
            .await?;

        let track_no: u32 = map.extract("Track")?.parse().unwrap();
        let duration = map.extract("TrackDuration")?;
        let elapsed = map.extract("RelTime")?;

        if track_no == 0
            || duration.eq_ignore_ascii_case("not_implemented")
            || elapsed.eq_ignore_ascii_case("not_implemented")
        {
            return Ok(None);
        }

        let metadata = map.extract("TrackMetaData")?;

        let duration = utils::duration_from_str(&duration)?;
        let elapsed = utils::duration_from_str(&elapsed)?;

        let doc = Document::parse(&metadata)?;
        let item = utils::find_root_node(&doc, "item", "Track Metadata")?;
        let track = Track::from_xml(item)?;

        Ok(Some(TrackInfo::new(track, track_no, duration, elapsed)))
    }

    // RENDERING_CONTROL

    pub async fn volume(&self) -> Result<u16> {
        let args = args! { "InstanceID": 0, "Channel": "Master" };
        self.action(RENDERING_CONTROL, "GetVolume", args)
            .await?
            .extract("CurrentVolume")
            .and_then(|x| x.parse().map_err(upnp::Error::invalid_response))
    }
    pub async fn set_volume(&self, volume: u16) -> Result<()> {
        let args = args! { "InstanceID": 0, "Channel": "Master", "DesiredVolume": volume };
        self.action(RENDERING_CONTROL, "SetVolume", args)
            .await
            .map(drop)
    }
    pub async fn set_volume_relative(&self, adjustment: i32) -> Result<u16> {
        let args = args! { "InstanceID": 0, "Channel": "Master", "Adjustment": adjustment };
        self.action(RENDERING_CONTROL, "SetRelativeVolume", args)
            .await?
            .extract("NewVolume")
            .and_then(|x| x.parse().map_err(upnp::Error::invalid_response))
    }

    pub async fn mute(&self) -> Result<bool> {
        let args = args! { "InstanceID": 0, "Channel": "Master" };
        self.action(RENDERING_CONTROL, "GetMute", args)
            .await?
            .extract("CurrentMute")
            .and_then(utils::parse_bool)
    }
    pub async fn set_mute(&self, mute: bool) -> Result<()> {
        let args = args! { "InstanceID": 0, "Channel": "Master", "DesiredMute": mute as u8 };
        self.action(RENDERING_CONTROL, "SetMute", args)
            .await
            .map(drop)
    }

    pub async fn bass(&self) -> Result<i16> {
        self.action(RENDERING_CONTROL, "GetBass", DEFAULT_ARGS)
            .await?
            .extract("CurrentBass")
            .and_then(|x| x.parse().map_err(upnp::Error::invalid_response))
    }
    pub async fn set_bass(&self, bass: i16) -> Result<()> {
        let args = args! { "InstanceID": 0, "DesiredBass": bass };
        self.action(RENDERING_CONTROL, "SetBass", args)
            .await
            .map(drop)
    }
    pub async fn treble(&self) -> Result<i16> {
        self.action(RENDERING_CONTROL, "GetTreble", DEFAULT_ARGS)
            .await?
            .extract("CurrentTreble")
            .and_then(|x| x.parse().map_err(upnp::Error::invalid_response))
    }
    pub async fn set_treble(&self, treble: i16) -> Result<()> {
        self.action(
            RENDERING_CONTROL,
            "SetTreble",
            args! { "InstanceID": 0, "DesiredTreble": treble },
        )
        .await
        .map(drop)
    }
    pub async fn loudness(&self) -> Result<bool> {
        let args = args! { "InstanceID": 0, "Channel": "Master" };
        self.action(RENDERING_CONTROL, "GetLoudness", args)
            .await?
            .extract("CurrentLoudness")
            .and_then(utils::parse_bool)
    }
    pub async fn set_loudness(&self, loudness: bool) -> Result<()> {
        let args =
            args! { "InstanceID": 0, "Channel": "Master", "DesiredLoudness": loudness as u8 };
        self.action(RENDERING_CONTROL, "SetLoudness", args)
            .await
            .map(drop)
    }

    // Queue
    pub async fn queue(&self) -> Result<Vec<Track>> {
        let args = args! { "QueueID": 0, "StartingIndex": 0, "RequestedCount": std::u32::MAX };
        let result = self
            .action(QUEUE, "Browse", args)
            .await?
            .extract("Result")?;

        Document::parse(&result)?
            .root()
            .first_element_child()
            .ok_or_else(|| upnp::Error::ParseError("Queue Response contains no children"))?
            .children()
            .filter(roxmltree::Node::is_element)
            .map(Track::from_xml)
            .collect()
    }

    pub async fn clear_queue(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "RemoveAllTracksFromQueue", DEFAULT_ARGS)
            .await
            .map(drop)
    }

    /// Returns a map of all discovered devices in the network to their respective information
    pub async fn group_topology(&self) -> Result<HashMap<String, Vec<SpeakerInfo>>> {
        let state = self
            .action(ZONE_GROUP_TOPOLOGY, "GetZoneGroupState", "")
            .await?
            .extract("ZoneGroupState")?;

        let doc = Document::parse(&state)?;
        let state = utils::find_root_node(&doc, "ZoneGroups", "Zone Group Topology")?;

        state
            .children()
            .filter(Node::is_element)
            .filter(|c| c.tag_name().name().eq_ignore_ascii_case("ZoneGroup"))
            .map(|node| {
                let coordinator = node
                    .attributes()
                    .iter()
                    .find(|a| a.name().eq_ignore_ascii_case("coordinator"))
                    .map(Attribute::value)
                    .ok_or_else(|| {
                        upnp::Error::XMLMissingElement("ZoneGroup".into(), "Coordinator".into())
                    })?
                    .to_string();

                let members = node
                    .children()
                    .filter(Node::is_element)
                    .filter(|c| c.tag_name().name().eq_ignore_ascii_case("ZoneGroupMember"))
                    .map(SpeakerInfo::from_xml)
                    .collect::<Result<Vec<_>>>()?;

                Ok((coordinator, members))
            })
            .collect()
    }

    /// From a group with a player.
    /// The uuid should not contain the `x-rincon:` part of the identifier.
    pub async fn join(&self, uuid: &str) -> Result<()> {
        let args = args! { "InstanceID": 0, "CurrentURI": format!("x-rincon:{}", uuid), "CurrentURIMetaData": "" };
        self.action(AV_TRANSPORT, "SetAV_TRANSPORTURI", args)
            .await
            .map(drop)
    }
    pub async fn leave(&self) -> Result<()> {
        self.action(
            AV_TRANSPORT,
            "BecomeCoordinatorOfStandaloneGroup",
            DEFAULT_ARGS,
        )
        .await
        .map(drop)
    }

    /// Execute some UPnP Action on the device.
    /// Panics if the service is not actually available.
    pub async fn action(
        &self,
        service: &URN,
        action: &str,
        payload: &str,
    ) -> Result<HashMap<String, String>> {
        self.0
            .find_service(service)
            .unwrap_or_else(|| panic!(format!("expected service '{}'", service)))
            .action(self.0.url(), action, payload)
            .await
    }
}
