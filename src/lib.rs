#![feature(async_await)]

mod datatypes;
mod speaker;
pub mod track;

pub use datatypes::{RepeatMode, SpeakerInfo};
pub use speaker::discover;
pub use speaker::Speaker;
pub use track::Track;
