//! SQLite adapter — local operational system of record (ARCH-01, ADR-0003).

pub mod migrations;
mod account_value;
mod ai;
mod allocation;
mod backtest;
mod cart;
mod classification;
mod distribution;
mod canonical;
mod wizard;
mod work_ticket;
mod field_decision;
mod issuer_pay;
mod magi;
mod plan;
mod regime;
mod roc_obs;
mod pd_settings;
mod payment_dates;
mod platform;
mod store;
mod trends;

pub use platform::LocalPlatform;
pub use store::{LocalDatabase, StorageError};
