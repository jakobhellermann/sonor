use crate::{
    args,
    track::{Track, TrackInfo},
    utils::{self, HashMapExt},
    RepeatMode, Result, Snapshot, SpeakerInfo,
};
use roxmltree::{Document, Node};
use rupnp::{ssdp::URN, Device};
use std::{collections::HashMap, net::Ipv4Addr};

pub(crate) const SONOS_URN: URN = URN::device("schemas-upnp-org", "ZonePlayer", 1);

const AV_TRANSPORT: &URN = &URN::service("schemas-upnp-org", "AVTransport", 1);
const DEVICE_PROPERTIES: &URN = &URN::service("schemas-upnp-org", "DeviceProperties", 1);
const RENDERING_CONTROL: &URN = &URN::service("schemas-upnp-org", "RenderingControl", 1);
const ZONE_GROUP_TOPOLOGY: &URN = &URN::service("schemas-upnp-org", "ZoneGroupTopology", 1);
const QUEUE: &URN = &URN::service("schemas-sonos-com", "Queue", 1);
const MUSIC_SERVICES: &URN = &URN::service("schemas-upnp-org", "MusicServices", 1);

const DEFAULT_ARGS: &str = "<InstanceID>0</InstanceID>";

#[derive(Debug, Clone)]
/// A sonos speaker, wrapping a UPnP-Device and providing user-oriented methods in an asynyronous
/// API.
pub struct Speaker(Device);

#[allow(missing_docs)]
impl Speaker {
    /// Creates a speaker from an already found UPnP-Device.
    /// Returns `None` when the URN type doesn't match the `schemas-upnp-org:device:ZonePlayer:1`,
    /// which is used by sonos devices.
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type() == &SONOS_URN {
            Some(Self(device))
        } else {
            None
        }
    }

    /// Creates a speaker from an IPv4 address.
    /// It returns `Ok(None)` when the device was found but isn't a sonos player.
    pub async fn from_ip(addr: Ipv4Addr) -> Result<Option<Self>> {
        let uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .expect("is always valid");

        Device::from_url(uri).await.map(Speaker::from_device)
    }

    pub fn device(&self) -> &Device {
        &self.0
    }

    pub async fn name(&self) -> Result<String> {
        self.action(DEVICE_PROPERTIES, "GetZoneAttributes", "")
            .await?
            .extract("CurrentZoneName")
    }

    pub async fn uuid(&self) -> Result<String> {
        let uuid = self
            ._zone_group_state()
            .await?
            .into_iter()
            .flat_map(|(_, speakers)| speakers)
            .find(|speaker_info| self.0.url() == speaker_info.location())
            .map(|speaker_info| speaker_info.uuid);

        Ok(uuid
            .expect("asked for zone group state but the speaker doesn't seem to be included there"))
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
        let res = self.action(AV_TRANSPORT, "Pause", DEFAULT_ARGS).await;
        match res {
            Ok(_) => Ok(()),
            Err(rupnp::Error::HttpErrorCode(code)) if code.as_u16() == 500 => Ok(()),
            Err(rupnp::Error::UPnPError(err)) if err.err_code() == 701 => Ok(()),
            Err(err) => Err(err),
        }
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

    pub async fn skip_to(&self, seconds: u32) -> Result<()> {
        let args = args! { "InstanceID": 0, "Unit": "REL_TIME", "Target": utils::seconds_to_str(seconds.into())};
        self.action(AV_TRANSPORT, "Seek", args).await.map(drop)
    }
    pub async fn skip_by(&self, seconds: i32) -> Result<()> {
        let args = args! { "InstanceID": 0, "Unit": "TIME_DELTA", "Target": utils::seconds_to_str(seconds.into())};
        self.action(AV_TRANSPORT, "Seek", args).await.map(drop)
    }
    /// The first track number is 1.
    pub async fn seek_track(&self, track_no: u32) -> Result<()> {
        let args = args! { "InstanceID": 0, "Unit": "TRACK_NR", "Target": track_no };
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
            _ => Err(rupnp::Error::invalid_response(
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

        // e.g. speaker was playing spotify, then spotify disconnected but sonos is still on
        // "x-sonos-vli"
        if duration.eq_ignore_ascii_case("not_implemented")
            || elapsed.eq_ignore_ascii_case("not_implemented")
        {
            return Ok(None);
        }

        let metadata = match map.remove("TrackMetaData") {
            Some(metadata) => metadata,
            None => return Ok(None),
        };

        let duration = utils::seconds_from_str(&duration)?;
        let elapsed = utils::seconds_from_str(&elapsed)?;

        let doc = Document::parse(&metadata)?;
        let item = utils::find_root_node(&doc, "item", "Track Metadata")?;
        let track = Track::from_xml(item)?;

        Ok(Some(TrackInfo::new(
            track, metadata, track_no, duration, elapsed,
        )))
    }

    // RENDERING_CONTROL

    pub async fn volume(&self) -> Result<u16> {
        let args = args! { "InstanceID": 0, "Channel": "Master" };
        self.action(RENDERING_CONTROL, "GetVolume", args)
            .await?
            .extract("CurrentVolume")
            .and_then(|x| x.parse().map_err(rupnp::Error::invalid_response))
    }
    pub async fn set_volume(&self, volume: u16) -> Result<()> {
        let args = args! { "InstanceID": 0, "Channel": "Master", "DesiredVolume": volume };
        self.action(RENDERING_CONTROL, "SetVolume", args)
            .await
            .map(drop)
    }
    pub async fn set_volume_relative(&self, adjustment: i16) -> Result<u16> {
        let args = args! { "InstanceID": 0, "Channel": "Master", "Adjustment": adjustment };
        self.action(RENDERING_CONTROL, "SetRelativeVolume", args)
            .await?
            .extract("NewVolume")
            .and_then(|x| x.parse().map_err(rupnp::Error::invalid_response))
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

    pub async fn bass(&self) -> Result<i8> {
        self.action(RENDERING_CONTROL, "GetBass", DEFAULT_ARGS)
            .await?
            .extract("CurrentBass")
            .and_then(|x| x.parse().map_err(rupnp::Error::invalid_response))
    }
    pub async fn set_bass(&self, bass: i8) -> Result<()> {
        let args = args! { "InstanceID": 0, "DesiredBass": bass };
        self.action(RENDERING_CONTROL, "SetBass", args)
            .await
            .map(drop)
    }
    pub async fn treble(&self) -> Result<i8> {
        self.action(RENDERING_CONTROL, "GetTreble", DEFAULT_ARGS)
            .await?
            .extract("CurrentTreble")
            .and_then(|x| x.parse().map_err(rupnp::Error::invalid_response))
    }
    pub async fn set_treble(&self, treble: i8) -> Result<()> {
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
            .ok_or_else(|| rupnp::Error::ParseError("Queue Response contains no children"))?
            .children()
            .filter(roxmltree::Node::is_element)
            .map(Track::from_xml)
            .collect()
    }

    // TODO test the next ones
    pub async fn remove_track(&self, track_no: u32) -> Result<()> {
        let args = args! { "InstanceID": 0, "ObjectID": format!("Q:0/{}", track_no + 1) };
        self.action(AV_TRANSPORT, "RemoveTrackFromQueue", args)
            .await
            .map(drop)
    }

    /// Enqueues a track at the end of the queue.
    pub async fn queue_end(&self, uri: &str, metadata: &str) -> Result<()> {
        let args = args! { "InstanceID": 0, "EnqueuedURI": uri, "EnqueuedURIMetaData": metadata, "DesiredFirstTrackNumberEnqueued": 0, "EnqueueAsNext": 0 };
        self.action(AV_TRANSPORT, "AddURIToQueue", args)
            .await
            .map(drop)
    }

    /// Enqueues a track as the next one.
    pub async fn queue_next(&self, uri: &str, metadata: &str) -> Result<()> {
        let args = args! { "InstanceID": 0, "EnqueuedURI": uri, "EnqueuedURIMetaData": metadata, "DesiredFirstTrackNumberEnqueued": 0, "EnqueueAsNext": 1 };
        self.action(AV_TRANSPORT, "AddURIToQueue", args)
            .await
            .map(drop)
    }

    pub async fn clear_queue(&self) -> Result<()> {
        self.action(AV_TRANSPORT, "RemoveAllTracksFromQueue", DEFAULT_ARGS)
            .await
            .map(drop)
    }

    pub(crate) async fn _zone_group_state(&self) -> Result<Vec<(String, Vec<SpeakerInfo>)>> {
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
            .map(|group| {
                let coordinator = utils::find_node_attribute(group, "Coordinator")?.to_string();
                let members = group
                    .children()
                    .filter(Node::is_element)
                    .filter(|c| c.tag_name().name().eq_ignore_ascii_case("ZoneGroupMember"))
                    .map(SpeakerInfo::from_xml)
                    .collect::<Result<Vec<_>>>()?;
                Ok((coordinator, members))
            })
            .collect()
    }

    /// Returns all groups in the system as a map from the group coordinators UUID to a list of [Speaker Info](struct.SpeakerInfo.html)s.
    pub async fn zone_group_state(&self) -> Result<HashMap<String, Vec<SpeakerInfo>>> {
        Ok(self._zone_group_state().await?.into_iter().collect())
    }

    /// Form a group with a player.
    /// The UUID should look like this: 'RINCON_000E5880EA7601400'.
    async fn join_uuid(&self, uuid: &str) -> Result<()> {
        let args = args! { "InstanceID": 0, "CurrentURI": format!("x-rincon:{}", uuid), "CurrentURIMetaData": "" };
        self.action(AV_TRANSPORT, "SetAVTransportURI", args)
            .await
            .map(drop)
    }

    /// Form a group with a player.
    /// Returns `false` when no player with that roomname exists.
    /// `roomname` is compared case insensitively.
    pub async fn join(&self, roomname: &str) -> Result<bool> {
        let topology = self._zone_group_state().await?;
        let uuid = topology
            .iter()
            .flat_map(|(_, speakers)| speakers)
            .find(|speaker_info| speaker_info.name().eq_ignore_ascii_case(roomname))
            .map(SpeakerInfo::uuid);

        if let Some(uuid) = uuid {
            self.join_uuid(uuid).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Leave the current group.
    /// Does nothing when the speaker already has no group.
    pub async fn leave(&self) -> Result<()> {
        self.action(
            AV_TRANSPORT,
            "BecomeCoordinatorOfStandaloneGroup",
            DEFAULT_ARGS,
        )
        .await
        .map(drop)
    }

    /// Set the transport URI for the speaker.
    pub async fn set_transport_uri(&self, uri: &str, metadata: &str) -> Result<()> {
        let args = args! { "InstanceID": 0, "CurrentURI": uri, "CurrentURIMetaData": metadata };
        self.action(AV_TRANSPORT, "SetAVTransportURI", args)
            .await
            .map(drop)
    }

    /// Get the current transport URI for the speaker.
    pub async fn transport_uri(&self) -> Result<Option<String>> {
        let uri = self
            .action(AV_TRANSPORT, "GetMediaInfo", DEFAULT_ARGS)
            .await?
            .remove("CurrentURI");
        Ok(uri)
    }

    #[allow(unused)]
    /// returns a map of lowercase service name to a tuple of (sid, capabilities, stype)
    async fn music_services(&self) -> Result<(Vec<u32>, HashMap<String, (u32, u32, u32)>)> {
        let mut map = self
            .action(MUSIC_SERVICES, "ListAvailableServices", "")
            .await?;
        let descriptor_list = map.extract("AvailableServiceDescriptorList")?;
        let service_type_list = map.extract("AvailableServiceTypeList")?;

        let available_services: Vec<u32> = service_type_list
            .split(',')
            .map(|x| x.parse())
            .collect::<Result<_, _>>()
            .map_err(rupnp::Error::invalid_response)?;

        let document = Document::parse(&descriptor_list)?;
        let services = utils::find_root_node(&document, "Services", "DescriptorList")?
            .children()
            .map(|node| -> Result<_> {
                let id = utils::find_node_attribute(node, "Id")?;
                let name = utils::find_node_attribute(node, "Name")?;
                let capabilities = utils::find_node_attribute(node, "Capabilities")?;

                let id = id.parse().map_err(rupnp::Error::invalid_response)?;
                let capabilities = capabilities
                    .parse()
                    .map_err(rupnp::Error::invalid_response)?;
                let s_type = id << (8 + 7);
                Ok((name.to_lowercase(), (id, capabilities, s_type)))
            })
            .collect::<Result<_, _>>()?;

        Ok((available_services, services))
    }

    /// Take a snapshot of the state the speaker is in right now.
    /// The saved information is the speakers volume, it's currently played song and were you were in the song.
    pub async fn snapshot(&self) -> Result<Snapshot> {
        Snapshot::from_speaker(&self).await
    }

    /// Applies a snapshot previously taken by the [snapshot](struct.Speaker.html#method.snapshot)-method.
    pub async fn apply(&self, snapshot: Snapshot) -> Result<()> {
        snapshot.apply(&self).await
    }

    /// Execute some UPnP Action on the device.
    /// Panics if the service is not actually available.
    /// A list of services, devices and actions of the 'ZonePlayer:1' standard can be found [here](https://github.com/jakobhellermann/sonos/tree/master/zoneplayer).
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
