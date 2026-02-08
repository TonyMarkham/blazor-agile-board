#![allow(dead_code, unused_imports)]

#[allow(clippy::module_inception)]
pub(crate) mod client;
pub(crate) mod error;

pub use client::Client;
pub use error::{ClientError, Result as CliClientResult};
