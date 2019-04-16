#![recursion_limit = "128"]
#![feature(async_await, await_macro, futures_api)]

use futures::prelude::*;

fn main() {
    tokio::run(
        async_main()
            .map_err(|e| eprintln!("{}", e))
            .boxed()
            .compat(),
    );
}

async fn async_main() -> Result<(), failure::Error> {
    let players = await!(sonos::discover(2))?;

    for player in &players {
        println!("{}", await!(player.get_name())?);
    }

    Ok(())
}
