use futures::Future;

fn main() {
    let f = sonos::player::discover(2)
        .map(|players| {
            players.into_iter().map(|player| player.get_name())
        })
        .and_then(futures::future::join_all)
        .map(|names| {
            println!("{:?}", names);
        })
        .map_err(|e| eprintln!("{}", e));

    tokio::run(f);
}