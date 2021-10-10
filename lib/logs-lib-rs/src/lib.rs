#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::map_unwrap_or)]

pub mod content;
pub mod event;

mod rpc;

pub mod submission {
    pub use crate::rpc::logs::submission::*;
}
