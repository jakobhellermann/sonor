use crate::track::{Track, TrackInfo};
use crate::{duration_to_str, parse_bool, HashMapExt, RepeatMode};
use futures::prelude::*;
use roxmltree::Document;
use std::{net::Ipv4Addr, time::Duration};
use upnp::{
    ssdp_client::search::{SearchTarget, URN},
    Device,
};

const SONOS_URN: URN<'static> = URN::device("schemas-upnp-org", "ZonePlayer", 1);

#[derive(Debug)]
pub struct Speaker(upnp::Device);

pub async fn discover(
    timeout: Duration,
) -> Result<impl Stream<Item = Result<Speaker, upnp::Error>>, upnp::Error> {
    Ok(upnp::discover(SearchTarget::URN(SONOS_URN), timeout)
        .await?
        .map_ok(|device| {
            Speaker::from_device(device).expect("searched for sonos urn but got something else")
        }))
}

macro_rules! upnp_action {
    ( $self:expr, $service:ident:$version:literal/$action:ident, $args:expr ) => {
        $self
            .0
            .find_service(&URN::service(
                "schemas-upnp-org",
                stringify!($service),
                $version,
            ))
            .expect(concat!(
                "sonos device doesn't have a ",
                stringify!($service),
                ':',
                $version,
                " service"
            ))
            .action($self.0.url(), stringify!($action), $args)
            .await
    };
}

macro_rules! args {
    ( $( $var:literal: $e:expr ),* ) => { &{
        let mut s = String::new();
        $(
            s.push_str(concat!("<", $var, ">"));
            s.push_str(&$e.to_string());
            s.push_str(concat!("</", $var, ">"));
        )*
        s
    } }
}

impl Speaker {
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type() == &SONOS_URN {
            Some(Self(device))
        } else {
            None
        }
    }
    pub async fn from_ip(addr: Ipv4Addr) -> Result<Option<Self>, upnp::Error> {
        let uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .expect("is always valid");

        Device::from_url(uri).await.map(Speaker::from_device)
    }

    pub async fn name(&self) -> Result<String, upnp::Error> {
        upnp_action!(self, DeviceProperties:1/GetZoneAttributes, "")?.extract("CurrentZoneName")
    }

    // AVTransport
    pub async fn stop(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Stop, args!{ "InstanceID": 0 }).map(|_| ())
    }
    pub async fn play(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Play, args!{ "InstanceID": 0, "Speed": 1 }).map(|_| ())
    }
    pub async fn pause(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Pause, args!{ "InstanceID": 0 }).map(|_| ())
    }
    pub async fn next(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Next, args!{ "InstanceID": 0 }).map(|_| ())
    }
    pub async fn previous(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Previous, args!{ "InstanceID": 0 }).map(|_| ())
    }

    pub async fn skip_to(&self, time: &Duration) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Seek, args! { "InstanceID": 0, "Unit": "REL_TIME", "Target": duration_to_str(time)})
            .map(|_| ())
    }
    pub async fn skip_by(&self, time: &Duration) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Seek, args! { "InstanceID": 0, "Unit": "TIME_DELTA", "Target": duration_to_str(time) })
            .map(|_| ())
    }
    pub async fn go_to_track(&self, track_no: u32) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/Seek, args! { "InstanceID": 0, "Unit": "TRACK_NR", "Target": track_no + 1})
            .map(|_| ())
    }

    async fn playback_mode(&self) -> Result<(RepeatMode, bool), upnp::Error> {
        let play_mode =
            upnp_action!(self, AVTransport:1/GetTransportSettings, args! { "InstanceID": 0 })?
                .extract("PlayMode")?;

        match play_mode.to_uppercase().as_str() {
            "NORMAL" => Ok((RepeatMode::None, false)),
            "REPEAT_ALL" => Ok((RepeatMode::All, false)),
            "REPEAT_ONE" => Ok((RepeatMode::One, false)),
            "SHUFFLE_NOREPEAT" => Ok((RepeatMode::None, true)),
            "SHUFFLE" => Ok((RepeatMode::All, true)),
            "SHUFFLE_REPEAT_ONE" => Ok((RepeatMode::One, true)),
            _ => Err(upnp::Error::InvalidResponse(Box::new(
                crate::datatypes::ParseRepeatModeError,
            ))),
        }
    }
    pub async fn repeat_mode(&self) -> Result<RepeatMode, upnp::Error> {
        self.playback_mode()
            .await
            .map(|(repeat_mode, _)| repeat_mode)
    }
    pub async fn shuffle(&self) -> Result<bool, upnp::Error> {
        self.playback_mode().await.map(|(_, shuffle)| shuffle)
    }

    async fn set_playback_mode(
        &self,
        repeat_mode: RepeatMode,
        shuffle: bool,
    ) -> Result<(), upnp::Error> {
        let playback_mode = match (repeat_mode, shuffle) {
            (RepeatMode::None, false) => "NORMAL",
            (RepeatMode::One, false) => "REPEAT_ONE",
            (RepeatMode::All, false) => "REPEAT_ALL",
            (RepeatMode::None, true) => "SHUFFLE_NOREPEAT",
            (RepeatMode::One, true) => "SHUFFLE_REPEAT_ONE",
            (RepeatMode::All, true) => "SHUFFLE",
        };
        upnp_action!(self, AVTransport:1/SetPlayMode, args! { "InstanceID": 0, "NewPlayMode": playback_mode })
            .map(|_| ())
    }
    pub async fn set_repeat_mode(&self, repeat_mode: RepeatMode) -> Result<(), upnp::Error> {
        self.set_playback_mode(repeat_mode, self.shuffle().await?)
            .await
    }
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<(), upnp::Error> {
        self.set_playback_mode(self.repeat_mode().await?, shuffle)
            .await
    }

    pub async fn crossfade(&self) -> Result<bool, upnp::Error> {
        upnp_action!(self, AVTransport:1/GetCrossfadeMode, args! { "InstanceID": 0 })?
            .extract("CrossfadeMode")
            .and_then(parse_bool)
    }
    pub async fn set_crossfade(&self, crossfade: bool) -> Result<(), upnp::Error> {
        let crossfade = crossfade as u8;
        upnp_action!(self, AVTransport:1/SetCrossfadeMode, args! { "InstanceID": 0, "CrossfadeMode": crossfade })
            .map(|_| ())
    }

    pub async fn is_playing(&self) -> Result<bool, upnp::Error> {
        upnp_action!(self, AVTransport:1/GetTransportInfo, args! { "InstanceID": 0 })?
            .extract("CurrentTransportState")
            .map(|x| x.eq_ignore_ascii_case("playing"))
    }

    pub async fn track(&self) -> Result<Option<TrackInfo>, upnp::Error> {
        let mut map = upnp_action!(self, AVTransport:1/GetPositionInfo, args! { "InstanceID": 0 })?;
        let track_no: u32 = map.extract("Track")?.parse().unwrap();
        if track_no == 0 {
            return Ok(None);
        }
        let metadata = map.extract("TrackMetaData")?;
        let duration = crate::duration_from_str(&map.extract("TrackDuration")?).unwrap();
        let elapsed = crate::duration_from_str(&map.extract("RelTime")?).unwrap(); // TODO

        let doc = Document::parse(&metadata)?;
        let item = crate::find_root_node(&doc, "item", "Track Metadata")?;
        let track = Track::from_xml(item)?;

        Ok(Some(TrackInfo::new(track, track_no, duration, elapsed)))
    }

    // RenderingControl

    pub async fn volume(&self) -> Result<u16, upnp::Error> {
        upnp_action!(self, RenderingControl:1/GetVolume, args! { "InstanceID": 0, "Channel": "Master" })?
            .extract("CurrentVolume")
            .and_then(|x| x.parse().map_err(|e| upnp::Error::InvalidResponse(Box::new(e))))
    }
    pub async fn set_volume(&self, volume: u16) -> Result<(), upnp::Error> {
        upnp_action!(self, RenderingControl:1/SetVolume, args! { "InstanceID": 0, "Channel": "Master", "DesiredVolume": volume })
            .map(|_| ())
    }
    pub async fn set_volume_relative(&self, adjustment: i32) -> Result<u16, upnp::Error> {
        upnp_action!(self, RenderingControl:1/SetRelativeVolume, args! { "InstanceID": 0, "Channel": "Master", "Adjustment": adjustment })?
            .extract("NewVolume")
            .and_then(|x| x.parse().map_err(|e| upnp::Error::InvalidResponse(Box::new(e))))
    }

    pub async fn mute(&self) -> Result<bool, upnp::Error> {
        upnp_action!(self, RenderingControl:1/GetMute, args! { "InstanceID": 0, "Channel": "Master" })?
            .extract("CurrentMute")
            .and_then(crate::parse_bool)
    }
    pub async fn set_mute(&self, mute: bool) -> Result<(), upnp::Error> {
        let mute = mute as u8;
        upnp_action!(self, RenderingControl:1/SetMute, args! { "InstanceID": 0, "Channel": "Master", "DesiredMute": mute })
            .map(|_| ())
    }

    pub async fn bass(&self) -> Result<i16, upnp::Error> {
        upnp_action!(self, RenderingControl:1/GetBass, args! { "InstanceID": 0 })?
            .extract("CurrentBass")
            .and_then(|x| {
                x.parse()
                    .map_err(|e| upnp::Error::InvalidResponse(Box::new(e)))
            })
    }
    pub async fn set_bass(&self, bass: i16) -> Result<(), upnp::Error> {
        upnp_action!(self, RenderingControl:1/SetBass, args! { "InstanceID": 0, "DesiredBass": bass })
            .map(|_| ())
    }
    pub async fn treble(&self) -> Result<i16, upnp::Error> {
        upnp_action!(self, RenderingControl:1/GetTreble, args! { "InstanceID": 0 })?
            .extract("CurrentTreble")
            .and_then(|x| {
                x.parse()
                    .map_err(|e| upnp::Error::InvalidResponse(Box::new(e)))
            })
    }
    pub async fn set_treble(&self, treble: i16) -> Result<(), upnp::Error> {
        upnp_action!(self, RenderingControl:1/SetTreble, args! { "InstanceID": 0, "DesiredTreble": treble })
            .map(|_| ())
    }
    pub async fn loudness(&self) -> Result<bool, upnp::Error> {
        upnp_action!(self, RenderingControl:1/GetLoudness, args! { "InstanceID": 0, "Channel": "Master" })?
            .extract("CurrentLoudness")
            .and_then(crate::parse_bool)
    }
    pub async fn set_loudness(&self, loudness: bool) -> Result<(), upnp::Error> {
        let loudness = loudness as u8;
        upnp_action!(self, RenderingControl:1/SetLoudness, args! { "InstanceID": 0, "Channel": "Master", "DesiredLoudness": loudness })
            .map(|_| ())
    }

    // Queue
    pub async fn queue(&self) -> Result<Vec<Track>, upnp::Error> {
        let mut map = self
            .0
            .find_service(&URN::service("schemas-sonos-com", "Queue", 1))
            .expect("sonos device doesn't have a Queue:1 service")
            .action(
                self.0.url(),
                "Browse",
                args! { "QueueID": 0, "StartingIndex": 0, "RequestedCount": std::u32::MAX },
            )
            .await?;
        let result = map.extract("Result")?;

        let doc = Document::parse(&result)?;

        doc.root()
            .first_element_child()
            .ok_or(upnp::Error::ParseError(
                "Queue Response contains no children",
            ))?
            .children()
            .filter(roxmltree::Node::is_element)
            .map(Track::from_xml)
            .collect()
    }

    pub async fn clear_queue(&self) -> Result<(), upnp::Error> {
        upnp_action!(self, AVTransport:1/RemoveAllTracksFromQueue, args! { "InstanceID": 0 })
            .map(|_| ())
    }

    /*
    // ZONEGROUPTOPOLOGY
    pub async fn group_topology(&self) -> Result<Vec<(String, Vec<SpeakerInfo>)>, upnp::Error> {
        let zonegrouptopology = self.zonegrouptopology();
        let state = zonegrouptopology.get_zone_group_state().await?;
        let mut state = Element::parse(state.as_bytes()).map_err(|_| upnp::Error::ParseError)?;

        let zone_groups = state
            .take_child("ZoneGroups")
            .map(|groups| groups.children)
            .unwrap_or_else(Vec::new);

        let mut groups = Vec::with_capacity(zone_groups.len());
        for mut group in zone_groups {
            let coordinator = group
                .attributes
                .remove("Coordinator")
                .ok_or(upnp::Error::ParseError)?;

            let mut zones = Vec::with_capacity(group.children.len());
            for zone in group.children {
                zones
                    .push(SpeakerInfo::from_xml(zone, &coordinator).ok_or(upnp::Error::ParseError)?)
            }

            groups.push((coordinator, zones))
        }

        Ok(groups)
    }
    pub async fn leave(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport
            .become_coordinator_of_standalone_group(0)
            .await?;
        Ok(())
    }
    pub async fn join(&self, uuid: String) -> Result<(), upnp::Error> {
        let to_join = format!("x-rincon:{}", uuid);
        self.set_transport_uri(to_join).await
    }
    */
}

#[cfg(test)]
mod tests {
    #[test]
    fn instance_id() {
        assert_eq!(
            args! { "InstanceID": 0, "Speed": 1 },
            "<InstanceID>0</InstanceID><Speed>1</Speed>"
        );
    }
}
