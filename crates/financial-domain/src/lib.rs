//! Financial domain layer.
//!
//! Entities, value objects, policies and deterministic calculations.
//! Must not depend on database, Tauri, network, broker or AI frameworks.

pub mod activity;
pub mod advisory;
pub mod allocation;
pub mod backtest;
pub mod calculator;
pub mod cart;
pub mod classification;
pub mod current_price;
pub mod declaration_lookback;
pub mod distribution;
pub mod div1;
pub mod dividend;
pub mod error;
pub mod income_plan;
pub mod lifetime;
pub mod lot;
pub mod magi;
pub mod money;
pub mod plan;
pub mod plan_review;
pub mod position;
pub mod regime;
pub mod roc;
pub mod schedule;
pub mod tax_projection;
pub mod trends;
pub mod updater;
pub mod week;
