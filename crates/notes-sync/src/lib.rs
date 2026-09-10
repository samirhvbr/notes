//! Causal synchronization, independent of filesystem adapters and transports.
//! A plan never mutates a note. Every application requires its observed heads.
mod journal;
mod plan;
pub mod store;
pub use journal::*;
pub use plan::*;
