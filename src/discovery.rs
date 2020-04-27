use crate::{
    speaker::{Speaker, SONOS_URN},
    Result,
};
use futures_util::stream::{FuturesUnordered, Stream, TryStreamExt};
use rupnp::Device;
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
///
/// # Example Usage
///
/// ```rust,no_run
/// # use futures::prelude::*;
/// # use std::time::Duration;
/// # async fn f() -> Result<(), sonos::Error> {
/// let mut devices = sonos::discover(Duration::from_secs(2)).await?;
///
/// while let Some(device) = devices.try_next().await? {
///     let name = device.name().await?;
///     println!("- {}", name);
/// }
/// # Ok(())
/// # };
pub async fn discover(timeout: Duration) -> Result<impl Stream<Item = Result<Speaker>>> {
    // this method searches for devices, but when it finds the first one it
    // uses its `.zone_group_state` to find the other devices in the network.

    let devices = rupnp::discover(&SONOS_URN.into(), timeout).await?;
    futures_util::pin_mut!(devices);

    let mut devices_iter = None;

    if let Some(device) = devices.try_next().await? {
        let iter = Speaker::from_device(device)
            .expect("searched for sonos urn but got something else")
            ._zone_group_state()
            .await?
            .into_iter()
            .flat_map(|(_, speakers)| speakers)
            .map(|speaker_info| {
                let url = speaker_info.location().parse();
                async {
                    let device = Device::from_url(url?).await?;
                    let speaker = Speaker::from_device(device);
                    Ok(speaker.expect("sonos action 'GetZoneGroupState' return non-sonos devices"))
                }
            });
        devices_iter = Some(iter);
    };

    Ok(devices_iter
        .into_iter()
        .flatten()
        .collect::<FuturesUnordered<_>>())
}

/// Search for a sonos speaker by its name.
///
/// # Example Usage
///
/// ```rust,no_run
/// # use futures::prelude::*;
/// # use std::time::Duration;
/// # async fn f() -> Result<(), sonos::Error> {
/// let speaker = sonos::find("your room name", Duration::from_secs(1)).await?
///     .expect("player exists");
/// assert_eq!(speaker.name().await?, "yoor room name");
/// # Ok(())
/// # };
pub async fn find(roomname: &str, timeout: Duration) -> Result<Option<Speaker>> {
    let mut devices = discover(timeout).await?;

    while let Some(device) = devices.try_next().await? {
        if device.name().await?.eq_ignore_ascii_case(roomname) {
            return Ok(Some(device));
        }
    }

    Ok(None)
}
