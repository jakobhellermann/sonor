use crate::speaker::{Speaker, SONOS_URN};
use crate::Result;
use upnp::Device;

use futures::prelude::*;
use futures::stream::FuturesUnordered;

use std::time::Duration;

// 1,408ms +/- 169ms for two devices in network
#[cfg(test)] // for benchmarking
pub(crate) async fn discover_simple(
    timeout: Duration,
) -> Result<impl Stream<Item = Result<Speaker>>> {
    let stream = upnp::discover(&SONOS_URN.into(), timeout)
        .await?
        .map_ok(Speaker::from_device)
        .map_ok(|device| device.expect("searched for sonos urn but got something else"));

    Ok(stream)
}

// 292ms +/- 191ms for two devices in network
/// discover sonos players on the network and stream their responses
pub async fn discover(timeout: Duration) -> Result<impl Stream<Item = Result<Speaker>>> {
    let devices = upnp::discover(&SONOS_URN.into(), timeout).await?;
    futures::pin_mut!(devices);

    let mut empty = None;
    let mut devices_iter = None;

    match devices.next().await {
        None => empty = Some(std::iter::empty()),
        Some(device) => {
            devices_iter = Some(
                Speaker::from_device(device?)
                    .expect("searched for sonos urn but got something else")
                    ._zone_group_state()
                    .await?
                    .into_iter()
                    .flat_map(|(_, speakers)| speakers)
                    .map(|speaker_info| {
                        let location = speaker_info.location().parse();
                        async {
                            let device = Device::from_url(location?).await?;
                            Ok(Speaker::from_device(device).expect(
                                "sonos action 'GetZoneGroupState' return non-sonos devices",
                            ))
                        }
                    }),
            )
        }
    };

    Ok(empty
        .into_iter()
        .flatten()
        .chain(devices_iter.into_iter().flatten())
        .collect::<FuturesUnordered<_>>())
}
