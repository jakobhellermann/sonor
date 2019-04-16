#![feature(async_await, await_macro, futures_api)]

mod speaker;
pub mod track;

pub use speaker::discover;
pub use speaker::Speaker;
pub use track::Track;
