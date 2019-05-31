use std::net::Ipv4Addr;
use std::num::NonZeroUsize;
use std::time::Duration;

use xmltree::Element;

use crate::{RepeatMode, SpeakerInfo};
use upnp::discovery;
use upnp::ssdp::SearchTarget;
use upnp::Device;
use upnp_zoneplayer1::avtransport1::{
    AVTransport1, CurrentPlayMode as PlayMode, SeekMode, TransportPlaySpeed, TransportState,
};
use upnp_zoneplayer1::deviceproperties1::DeviceProperties1;
use upnp_zoneplayer1::renderingcontrol1::{Channel, MuteChannel, RenderingControl1};
use upnp_zoneplayer1::GroupRenderingControl1;
use upnp_zoneplayer1::Queue1;
use upnp_zoneplayer1::ZoneGroupTopology1;

use crate::track::{self, duration_from_str, Track, TrackInfo};

const SONOS_URN: &str = "schemas-upnp-org:device:ZonePlayer:1";

#[derive(Debug)]
pub struct Speaker {
    device: upnp::Device,
}

pub async fn discover(timeout: Duration) -> Result<Vec<Speaker>, upnp::Error> {
    Ok(
        discovery::discover(SearchTarget::URN(SONOS_URN), timeout)
            .await?
            .into_iter()
            .map(|device| {
                Speaker::from_device(device).expect("searched for sonos urn but got something else")
            })
            .collect(),
    )
}

impl Speaker {
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type().ends_with(SONOS_URN) {
            Some(Self { device })
        } else {
            None
        }
    }
    pub async fn from_ip(addr: Ipv4Addr) -> Result<Option<Self>, upnp::Error> {
        let uri: hyper::Uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .expect("is always valid");

        Device::from_url(uri).await.map(Speaker::from_device)
    }

    // SERVICES
    fn rendering_control(&self) -> RenderingControl1 {
        RenderingControl1::from_device(&self.device)
            .expect("sonos device does not have a RenderingControl1 service")
    }
    fn queue(&self) -> Queue1 {
        Queue1::from_device(&self.device).expect("sonos device does not have a Queue1 service")
    }
    fn avtransport(&self) -> AVTransport1 {
        AVTransport1::from_device(&self.device)
            .expect("sonos device does not have a AVTransport1 service")
    }
    fn deviceproperties(&self) -> DeviceProperties1 {
        DeviceProperties1::from_device(&self.device)
            .expect("sonos device does not have a DeviceProperties1 service")
    }
    #[allow(unused)]
    fn grouprenderingcontrol(&self) -> GroupRenderingControl1 {
        GroupRenderingControl1::from_device(&self.device)
            .expect("sonos device does not have a GroupRenderingControl1 servive")
    }
    fn zonegrouptopology(&self) -> ZoneGroupTopology1 {
        ZoneGroupTopology1::from_device(&self.device)
            .expect("sonos device does not have a ZoneGroupTopology1 servive")
    }

    // AVTRANSPORT
    // action![avtransport, (0, TransportPlaySpeed::_1)]
    pub async fn play(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.play(0, TransportPlaySpeed::_1).await
    }
    pub async fn pause(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.pause(0).await
    }
    pub async fn stop(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.stop(0).await
    }
    pub async fn previous(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.previous(0).await
    }
    pub async fn next(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.next(0).await
    }

    /// returns repeat mode and shuffle
    pub async fn playback_mode(&self) -> Result<(RepeatMode, bool), upnp::Error> {
        let avtransport = self.avtransport();
        let (play_mode, _) = avtransport.get_transport_settings(0).await?;

        let (repeat, shuffle) = match play_mode {
            PlayMode::NORMAL => (RepeatMode::NONE, false),
            PlayMode::REPEAT_ALL => (RepeatMode::ALL, false),
            PlayMode::REPEAT_ONE => (RepeatMode::ONE, false),
            PlayMode::SHUFFLE_NOREPEAT => (RepeatMode::NONE, true),
            PlayMode::SHUFFLE => (RepeatMode::ALL, true),
            PlayMode::SHUFFLE_REPEAT_ONE => (RepeatMode::ONE, true),
        };

        Ok((repeat, shuffle))
    }
    pub async fn set_playback_mode(
        &self,
        repeat: RepeatMode,
        shuffle: bool,
    ) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        let playmode = match (repeat, shuffle) {
            (RepeatMode::NONE, false) => PlayMode::NORMAL,
            (RepeatMode::ONE, false) => PlayMode::REPEAT_ONE,
            (RepeatMode::ALL, false) => PlayMode::REPEAT_ALL,
            (RepeatMode::NONE, true) => PlayMode::SHUFFLE_NOREPEAT,
            (RepeatMode::ONE, true) => PlayMode::SHUFFLE_REPEAT_ONE,
            (RepeatMode::ALL, true) => PlayMode::SHUFFLE,
        };
        avtransport.set_play_mode(0, playmode).await
    }
    pub async fn crossfade(&self) -> Result<bool, upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.get_crossfade_mode(0).await.map(Into::into)
    }
    pub async fn set_crossfade(&self, crossfade: bool) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.set_crossfade_mode(0, crossfade.into()).await
    }
    pub async fn seek_track(&self, track: NonZeroUsize) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport
            .seek(0, SeekMode::TRACK_NR, track.to_string())
            .await
    }
    pub async fn seek_time(&self, time: Duration) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport
            .seek(0, SeekMode::REL_TIME, track::duration_to_str(&time))
            .await
    }
    pub async fn skip_time(&self, time: Duration) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport
            .seek(0, SeekMode::TIME_DELTA, track::duration_to_str(&time))
            .await
    }
    pub async fn transport_state(&self) -> Result<TransportState, upnp::Error> {
        let avtransport = self.avtransport();
        let (transport_state, _transport_status, _speed) =
            avtransport.get_transport_info(0).await?;
        Ok(transport_state)
    }
    pub async fn set_transport_uri(&self, uri: String) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport
            .set_avtransport_uri(0, uri, Default::default())
            .await
    }

    pub async fn track(&self) -> Result<Option<TrackInfo>, upnp::Error> {
        let avtransport = self.avtransport();
        let (track_no, duration, metadata, _, played, _, _, _) =
            avtransport.get_position_info(0).await?;

        if let Ok(mut track) = Element::parse(metadata.as_bytes()) {
            let item = track.take_child("item").ok_or(upnp::Error::ParseError)?;
            let track = Track::from_xml(item).ok_or(upnp::Error::ParseError)?;

            if let Some(duration) = duration_from_str(&duration) {
                if let Some(played) = duration_from_str(&played) {
                    return Ok(Some(TrackInfo::new(track, track_no, duration, played)));
                }
            }
            return Err(upnp::Error::ParseError);
        } else {
            return Ok(None);
        }
    }

    // RENDERINGCONTROL
    pub async fn set_volume(&self, volume: u16) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .set_volume(0, Channel::Master, volume)
            .await
    }
    pub async fn set_volume_relative(&self, adjustment: i32) -> Result<u16, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .set_relative_volume(0, Channel::Master, adjustment)
            .await
    }
    pub async fn get_volume(&self) -> Result<u16, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control.get_volume(0, Channel::Master).await
    }

    pub async fn get_mute(&self) -> Result<bool, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .get_mute(0, MuteChannel::Master)
            .await
            .map(Into::into)
    }
    pub async fn set_mute(&self, mute: bool) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .set_mute(0, MuteChannel::Master, mute.into())
            .await
    }

    pub async fn set_bass(&self, bass: i16) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control.set_bass(0, bass).await
    }
    pub async fn get_bass(&self) -> Result<i16, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control.get_bass(0).await
    }
    pub async fn set_treble(&self, treble: i16) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control.set_treble(0, treble).await
    }
    pub async fn get_treble(&self) -> Result<i16, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control.get_treble(0).await
    }
    pub async fn set_loudness(&self, loudness: bool) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .set_loudness(0, Channel::Master, loudness.into())
            .await
    }
    pub async fn get_loudness(&self) -> Result<bool, upnp::Error> {
        let rendering_control = self.rendering_control();
        rendering_control
            .get_loudness(0, Channel::Master)
            .await
            .map(Into::into)
    }

    // DEVICEPROPERTIES
    pub async fn get_name(&self) -> Result<String, upnp::Error> {
        let deviceproperties = self.deviceproperties();
        let (name, _, _) = deviceproperties.get_zone_attributes().await?;
        Ok(name)
    }

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

    // QUEUE
    pub async fn get_queue(&self) -> Result<Vec<Track>, upnp::Error> {
        let queue_svc = self.queue();
        let (result, _number_returned, _total_matches, _update_id) =
            queue_svc.browse(0, 0, std::u32::MAX).await?;

        let queue = Element::parse(result.as_bytes()).map_err(|_| upnp::Error::ParseError)?;
        let mut tracks = Vec::with_capacity(queue.children.len());
        for track in queue.children {
            tracks.push(Track::from_xml(track).ok_or(upnp::Error::ParseError)?);
        }
        Ok(tracks)
    }
    pub async fn clear_queue(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        avtransport.remove_all_tracks_from_queue(0).await
    }
}
