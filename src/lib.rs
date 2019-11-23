#![warn(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    missing_debug_implementations,
    missing_docs
)]

//! This crate is a Sonos controller library written in Rust.
//! It operates asynchronously and aims for a simple to use yet powerful API.
//!
//! # Example
//! ```rust,no_run
//! # use futures::prelude::*;
//! # use std::time::Duration;
//! # async_std::task::block_on(async {
//! let speaker = sonos::find("your room name", Duration::from_secs(2)).await?
//!     .expect("room exists");
//!
//! println!("The volume is currently at {}", speaker.volume().await?);
//!
//! match speaker.track().await? {
//!     Some(track_info) => println!("- Currently playing '{}", track_info.track()),
//!     None => println!("- No track currently playing"),
//! }
//!
//! speaker.clear_queue().await?;
//!
//! speaker.join("some other room").await?;
//!
//! # Ok::<_, sonos::Error>(())
//! # });
//! ```
//! For a full list of actions implemented, look at the [Speaker](struct.Speaker.html) docs.
//!
//! If your use case isn't covered, this crate also exposes the raw UPnP Action API
//! [here](struct.Speaker.html#method.action).
//! It can be used like this:
//! ```rust,no_run
//! # use futures::prelude::*;
//! # use std::time::Duration;
//! use sonos::URN;
//! # async_std::task::block_on(async {
//! # let speaker = sonos::find("your room name", Duration::from_secs(2)).await?.expect("room exists");
//!
//! let service = URN::service("schemas-upnp-org", "GroupRenderingControl", 1);
//! let args = sonos::args! {
//!     "InstanceID": 0
//! };
//! let response = speaker.action(&service, "GetGroupMute", args).await?;
//!
//! println!("{}", response["CurrentMute"]);
//!
//! # Ok::<_, sonos::Error>(())
//! # });
//! ```

mod datatypes;
mod discovery;
mod speaker;
mod track;
mod utils;

pub use datatypes::{RepeatMode, SpeakerInfo};
pub use discovery::{discover, find};
pub use speaker::Speaker;
pub use track::{Track, TrackInfo};

pub use upnp;
pub use upnp::ssdp::URN;
pub use upnp::Error;

type Result<T> = std::result::Result<T, Error>;
