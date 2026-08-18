//! SQLite adapter — local operational system of record (ARCH-01, ADR-0003).

pub mod migrations;
mod ai;
mod allocation;
mod backtest;
mod cart;
mod classification;
mod canonical;
mod magi;
mod plan;
mod platform;
mod store;

pub use platform::LocalPlatform;
pub use store::{LocalDatabase, StorageError};
