use std::net::Ipv4Addr;
use std::time::Duration;

use xmltree::Element;

use upnp::discovery;
use upnp::ssdp::{header::ST, FieldMap};
use upnp::Device;

use upnp_zoneplayer1::avtransport1::{
    AVTransport1, CurrentPlayMode, SeekMode, TransportPlaySpeed, TransportState,
};
use upnp_zoneplayer1::deviceproperties1::DeviceProperties1;
use upnp_zoneplayer1::queue1::Queue1;
use upnp_zoneplayer1::renderingcontrol1::{Channel, MuteChannel, RenderingControl1};

use crate::track::{self, duration_from_str, Track, TrackInfo};

const SONOS_URN: &str = "schemas-upnp-org:device:ZonePlayer:1";

#[derive(Debug)]
pub struct Speaker {
    device: upnp::Device,
}

pub async fn discover(timeout: u8) -> Result<Vec<Speaker>, upnp::Error> {
    Ok(await!(discovery::discover(
        ST::Target(FieldMap::URN(SONOS_URN.to_string())),
        timeout
    ))?
    .into_iter()
    .map(|device| {
        Speaker::from_device(device).expect("searched for sonos urn but got something else")
    })
    .collect())
}

#[derive(Debug)]
pub enum RepeatMode {
    NONE,
    ONE,
    ALL,
}

impl Speaker {
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type().ends_with(SONOS_URN) {
            Some(Self { device })
        } else {
            None
        }
    }
    pub async fn from_ip(addr: Ipv4Addr) -> Result<Option<Self>, hyper::Error> {
        let uri: hyper::Uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .unwrap();

        await!(Device::from_url(uri)).map(Speaker::from_device)
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

    // AVTRANSPORT
    pub async fn play(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.play(0, TransportPlaySpeed::_1))
    }
    pub async fn pause(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.pause(0))
    }
    pub async fn stop(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.stop(0))
    }
    pub async fn previous(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.previous(0))
    }
    pub async fn next(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.next(0))
    }
    pub async fn playback_mode(&self) -> Result<(RepeatMode, bool, bool), upnp::Error> {
        let avtransport = self.avtransport();
        let (play_mode, _) = await!(avtransport.get_transport_settings(0))?;
        let crossfade = await!(avtransport.get_crossfade_mode(0))?.into();

        let (repeat, shuffle) = match play_mode {
            CurrentPlayMode::NORMAL => (RepeatMode::NONE, false),
            CurrentPlayMode::REPEAT_ALL => (RepeatMode::ALL, false),
            CurrentPlayMode::REPEAT_ONE => (RepeatMode::ONE, false),
            CurrentPlayMode::SHUFFLE_NOREPEAT => (RepeatMode::NONE, true),
            CurrentPlayMode::SHUFFLE => (RepeatMode::ALL, true),
            CurrentPlayMode::SHUFFLE_REPEAT_ONE => (RepeatMode::ONE, true),
        };

        Ok((repeat, shuffle, crossfade))
    }
    pub async fn seek_track(&self, track: usize) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.seek(0, SeekMode::TRACK_NR, track.to_string()))
    }
    pub async fn seek_time(&self, time: Duration) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.seek(0, SeekMode::REL_TIME, track::duration_to_str(&time)))
    }
    pub async fn skip_time(&self, time: Duration) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.seek(0, SeekMode::TIME_DELTA, track::duration_to_str(&time)))
    }
    pub async fn transport_state(&self) -> Result<TransportState, upnp::Error> {
        let avtransport = self.avtransport();
        let (transport_state, _transport_status, _speed) =
            await!(avtransport.get_transport_info(0))?;
        Ok(transport_state)
    }
    pub async fn set_uri(&self, uri: String) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.set_avtransport_uri(0, uri, Default::default()))
    }

    pub async fn track(&self) -> Result<Option<TrackInfo>, upnp::Error> {
        let avtransport = self.avtransport();
        let (track_no, duration, metadata, _, played, _, _, _) =
            await!(avtransport.get_position_info(0))?;

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
        await!(rendering_control.set_volume(0, Channel::Master, volume))
    }
    pub async fn get_volume(&self) -> Result<u16, upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.get_volume(0, Channel::Master))
    }

    pub async fn get_mute(&self) -> Result<bool, upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.get_mute(0, MuteChannel::Master)).map(From::from)
    }
    pub async fn set_mute(&self, mute: bool) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.set_mute(0, MuteChannel::Master, mute.into()))
    }

    pub async fn set_bass(&self, bass: i16) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.set_bass(0, bass))
    }
    pub async fn get_bass(&self) -> Result<i16, upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.get_bass(0))
    }
    pub async fn set_treble(&self, treble: i16) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.set_treble(0, treble))
    }
    pub async fn get_treble(&self) -> Result<i16, upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.get_treble(0))
    }
    pub async fn set_loudness(&self, loudness: bool) -> Result<(), upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.set_loudness(0, Channel::Master, loudness.into()))
    }
    pub async fn get_loudness(&self) -> Result<bool, upnp::Error> {
        let rendering_control = self.rendering_control();
        await!(rendering_control.get_loudness(0, Channel::Master)).map(Into::into)
    }

    // DEVICEPROPERTIES
    pub async fn get_name(&self) -> Result<String, upnp::Error> {
        let deviceproperties = self.deviceproperties();
        let (name, _, _) = await!(deviceproperties.get_zone_attributes())?;
        Ok(name)
    }

    // QUEUE
    pub async fn get_queue(&self) -> Result<Vec<Track>, upnp::Error> {
        let queue_svc = self.queue();
        let (result, _number_returned, _total_matches, _update_id) =
            await!(queue_svc.browse(0, 0, std::u32::MAX))?;

        let queue = Element::parse(result.as_bytes()).map_err(|_| upnp::Error::ParseError)?;
        let mut tracks = Vec::with_capacity(queue.children.len());
        for track in queue.children {
            tracks.push(Track::from_xml(track).ok_or(upnp::Error::ParseError)?);
        }
        Ok(tracks)
    }
    pub async fn clear_queue(&self) -> Result<(), upnp::Error> {
        let avtransport = self.avtransport();
        await!(avtransport.remove_all_tracks_from_queue(0))
    }
}
