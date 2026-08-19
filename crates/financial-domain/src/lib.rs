//! Financial domain layer.
//!
//! Entities, value objects, policies and deterministic calculations.
//! Must not depend on database, Tauri, network, broker or AI frameworks.

pub mod activity;
pub mod advisory;
pub mod allocation;
pub mod backtest;
pub mod cart;
pub mod classification;
pub mod dividend;
pub mod error;
pub mod lot;
pub mod magi;
pub mod money;
pub mod plan;
pub mod position;
pub mod tax_projection;
pub mod updater;
pub mod week;
