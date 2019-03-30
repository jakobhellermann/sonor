#![allow(unused)]

use sonos::Player;
use upnp::Device;

use futures::Future;
use tokio::runtime::Runtime;

use std::net::Ipv4Addr;

fn main() -> Result<(), failure::Error> {
    let mut rt = Runtime::new()?;

    let player: Player = rt
        .block_on(Player::from_ip(Ipv4Addr::new(192, 168, 2, 49)))?
        .unwrap();

    match rt.block_on(player.get_transport_state()) {
        Ok(val) => println!("{}", val),
        Err(e) => eprintln!("{}", e),
    }

    Ok(())
}
