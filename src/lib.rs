#![feature(test)]

mod datatypes;
mod discovery;
mod speaker;
pub mod track;
mod utils;

pub use datatypes::{RepeatMode, SpeakerInfo};
pub use discovery::discover;
pub use speaker::Speaker;
pub use track::Track;

pub use upnp;

type Result<T> = std::result::Result<T, upnp::Error>;
