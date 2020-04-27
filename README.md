This crate is a Sonos controller library written in Rust.
It operates asynchronously and aims for a simple to use yet powerful API.

# Example
```rust
let speaker = sonos::find("your room name", Duration::from_secs(2)).await?
    .expect("room exists");

println!("The volume is currently at {}", speaker.volume().await?);

match speaker.track().await? {
    Some(track_info) => println!("- Currently playing '{}", track_info.track()),
    None => println!("- No track currently playing"),
}

speaker.clear_queue().await?;

speaker.join("some other room").await?;
```
For a full list of actions implemented, look at the [Speaker](struct.Speaker.html) docs.

If your use case isn't covered, this crate also exposes the raw UPnP Action API
[here](struct.Speaker.html#method.action).
It can be used like this:
```rust
use sonos::URN;

let service = URN::service("schemas-upnp-org", "GroupRenderingControl", 1);
let args = "<InstanceID>0</InstanceID>";
let response = speaker.action(&service, "GetGroupMute", args).await?;

println!("{}", response["CurrentMute"]);
```

