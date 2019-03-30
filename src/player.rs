#![allow(unused)]
use futures::{future, Future};

use std::net::Ipv4Addr;
use std::time::Duration;
use upnp::discovery;
use upnp::ssdp::{header::ST, FieldMap};
use upnp::Device;
use upnp::Error;
use xmltree::Element;

use upnp_zoneplayer1::avtransport1::{AVTransport1, SeekMode, TransportPlaySpeed, TransportState};
use upnp_zoneplayer1::deviceproperties1::DeviceProperties1;
use upnp_zoneplayer1::queue1::Queue1;
use upnp_zoneplayer1::renderingcontrol1::{Channel, MuteChannel, RenderingControl1};

const SONOS_URN: &str = "schemas-upnp-org:device:ZonePlayer:1";

#[derive(Debug)]
pub struct Player {
    device: upnp::Device,
}

pub fn discover(timeout: u8) -> impl Future<Item = Vec<Player>, Error = Error> {
    discovery::discover(ST::Target(FieldMap::URN(SONOS_URN.to_string())), timeout).map(|devices| {
        devices
            .into_iter()
            .map(|device| {
                Player::from_device(device).expect("searched for sonos urn but got something else")
            })
            .collect()
    })
}

fn seconds_to_sonos_fmt(seconds_total: u64) -> String {
    const SECS_PER_MINUTE: u64 = 60;
    const MINS_PER_HOUR: u64 = 60;
    const SECS_PER_HOUR: u64 = 3600;

    let seconds = seconds_total % SECS_PER_MINUTE;
    let minutes = (seconds_total / SECS_PER_MINUTE) % MINS_PER_HOUR;
    let hours = seconds_total / SECS_PER_HOUR;

    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
}

impl Player {
    pub fn from_device(device: Device) -> Option<Self> {
        if device.device_type().ends_with(SONOS_URN) {
            Some(Self { device })
        } else {
            None
        }
    }
    pub fn from_ip(addr: Ipv4Addr) -> impl Future<Item = Option<Self>, Error = hyper::Error> {
        let uri: hyper::Uri = format!("http://{}:1400/xml/device_description.xml", addr)
            .parse()
            .unwrap();

        Device::from_url(uri).map(Player::from_device)
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
    pub fn play(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().play(0, TransportPlaySpeed::_1)
    }
    pub fn pause(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().pause(0)
    }
    pub fn stop(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().stop(0)
    }
    pub fn previous(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().previous(0)
    }
    pub fn next(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().next(0)
    }
    pub fn seek_track(&self, track: usize) -> impl Future<Item = (), Error = Error> {
        self.avtransport()
            .seek(0, SeekMode::TRACK_NR, &track.to_string())
    }
    pub fn seek_time(&self, time: &Duration) -> impl Future<Item = (), Error = Error> {
        self.avtransport()
            .seek(0, SeekMode::REL_TIME, &seconds_to_sonos_fmt(time.as_secs()))
    }
    pub fn skip_time(&self, seconds: u64) -> impl Future<Item = (), Error = Error> {
        self.avtransport()
            .seek(0, SeekMode::TIME_DELTA, &seconds_to_sonos_fmt(seconds))
    }
    pub fn get_transport_state(&self) -> impl Future<Item = TransportState, Error = Error> {
        self.avtransport()
            .get_transport_info(0)
            .map(|(transport_state, transport_status, speed)| transport_state)
    }
    pub fn set_uri(&self, uri: &str) -> impl Future<Item = (), Error = Error> {
        self.avtransport().set_avtransport_uri(0, uri, "")
    }
    pub fn track(&self) -> impl Future<Item = (), Error = Error> {
        self.avtransport().get_position_info(0).map(
            |(track_no, duration, metadata, uri, rel_time, _abs_time, _rel_count, _abs_count)| {
                let metadata =
                    String::from_utf8(marksman_escape::Unescape::new(metadata.bytes()).collect())
                        .unwrap();
                println!(
                    "{}. {} ({}/{}) --- {}",
                    track_no, uri, rel_time, duration, metadata
                );

                let track = Element::parse(metadata.as_bytes()).unwrap();
                let item = track.get_child("item").unwrap();
                println!("{:?}", item.get_child("title").unwrap().text);
            },
        )
    }

    // RENDERINGCONTROL
    pub fn get_volume(&self) -> impl Future<Item = u16, Error = Error> {
        self.rendering_control().get_volume(0, Channel::Master)
    }
    pub fn set_volume(&self, volume: u16) -> impl Future<Item = (), Error = Error> {
        self.rendering_control()
            .set_volume(0, Channel::Master, volume)
    }
    pub fn get_mute(&self) -> impl Future<Item = bool, Error = Error> {
        self.rendering_control()
            .get_mute(0, MuteChannel::Master)
            .map(Into::into)
    }
    pub fn set_mute(&self, muted: bool) -> impl Future<Item = (), Error = Error> {
        self.rendering_control()
            .set_mute(0, MuteChannel::Master, muted.into())
    }
    pub fn get_bass(&self) -> impl Future<Item = i16, Error = Error> {
        self.rendering_control().get_bass(0)
    }
    pub fn set_bass(&self, bass: i16) -> impl Future<Item = (), Error = Error> {
        self.rendering_control().set_bass(0, bass)
    }
    pub fn get_treble(&self) -> impl Future<Item = i16, Error = Error> {
        self.rendering_control().get_treble(0)
    }
    pub fn set_treble(&self, treble: i16) -> impl Future<Item = (), Error = Error> {
        self.rendering_control().set_treble(0, treble)
    }

    // DEVICEPROPERTIES
    pub fn get_name(&self) -> impl Future<Item = String, Error = Error> {
        self.deviceproperties()
            .get_zone_attributes()
            .map(|(name, _, _)| name)
    }
}
