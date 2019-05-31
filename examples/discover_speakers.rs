#![feature(async_await, await_macro)]
#![recursion_limit = "128"]

#[runtime::main(runtime_tokio::Tokio)]
async fn main() -> Result<(), failure::Error> {
    for player in sonos::discover(2).await? {
        println!("{}", player.get_name().await?);
    }

    Ok(())
}
