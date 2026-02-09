//! pm-cli library
//!
//! This module exports the HTTP client for use in tests and other crates.

pub(crate) mod cli;
pub(crate) mod client;
pub(crate) mod commands;
pub(crate) mod comment_commands;
pub(crate) mod dependency_commands;
pub(crate) mod project_commands;
pub(crate) mod sprint_commands;
pub(crate) mod swim_lane_commands;
pub(crate) mod work_item_commands;

#[cfg(test)]
mod tests;

pub use client::{CliClientResult, Client, ClientError};
