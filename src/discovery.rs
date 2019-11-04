use crate::speaker::{Speaker, SONOS_URN};
use crate::Result;
use upnp::Device;

use futures::prelude::*;
use futures::stream::FuturesUnordered;

use std::time::Duration;

// 1,408ms +/- 169ms for two devices in network
/*pub(crate) async fn discover_simple(
    timeout: Duration,
) -> Result<impl Stream<Item = Result<Speaker>>> {
    let stream = upnp::discover(&SONOS_URN.into(), timeout)
        .await?
        .map_ok(Speaker::from_device)
        .map_ok(|device| device.expect("searched for sonos urn but got something else"));

    Ok(stream)
}*/

// 292ms +/- 191ms for two devices in network
/// Discover sonos players on the network.
/// The ergonomics will get nicer when there are `for await`-loops in rust, but until then we have
/// to use `pin_mut` or the `futures-async-stream`-crate.
///
/// # Example Usage
///
/// ```rust,no_run
/// # use futures::prelude::*;
/// # use std::time::Duration;
/// # async_std::task::block_on(async {
/// let devices = sonos::discover(Duration::from_secs(2)).await?;
///
/// futures::pin_mut!(devices);
/// while let Some(device) = devices.next().await {
///     let device = device?;
///     let name = device.name().await?;
///     println!("- {}", name);
/// }
///
/// # Ok::<_, sonos::Error>(())
/// # });
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

/// Search for a sonos speaker by its name.
///
/// # Example Usage
///
/// ```rust,no_run
/// # use futures::prelude::*;
/// # use std::time::Duration;
/// # async_std::task::block_on(async {
/// let speaker = sonos::find("your room name", Duration::from_secs(1)).await?
///     .expect("player exists");
/// assert_eq!(speaker.name().await?, "yoor room name");
/// # Ok::<_, sonos::Error>(())
/// # });
pub async fn find(roomname: &str, timeout: Duration) -> Result<Option<Speaker>> {
    let devices = discover(timeout).await?;
    futures::pin_mut!(devices);

    while let Some(device) = devices.next().await {
        let device = device?;
        if device.name().await?.eq_ignore_ascii_case(roomname) {
            return Ok(Some(device));
        }
    }

    Ok(None)
}
