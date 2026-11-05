//! tidydrop-core: all file logic lives here so the desktop app and the CLI share it.
//!
//! Flow: [`scanner::scan`] -> [`plan::build_plan`] (rules, optional [`ai`]) ->
//! user reviews -> [`apply::apply_plan`] (journaled moves) -> [`journal::undo`].

#![cfg_attr(test, allow(clippy::field_reassign_with_default))]

pub mod ai;
pub mod apply;
pub mod config;
pub mod error;
pub mod journal;
pub mod paths;
pub mod pipeline;
pub mod plan;
pub mod rules;
pub mod scanner;
pub mod snippet;
pub mod stats;
pub mod undo;

pub use error::{Error, Result};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
