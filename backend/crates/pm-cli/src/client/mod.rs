pub(crate) mod client;
pub(crate) mod error;

pub use client::Client;
pub use error::{ClientError, Result as CliClientResult};
