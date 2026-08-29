//! Application services layer.
//!
//! Use cases, workflows, transactions, approvals and orchestration.
//! Coordinates domain components through ports; contains no UI logic.

pub mod contracts;
pub mod golden;
pub mod handoff;
pub mod ports;
pub mod production_seed;
pub mod queries;
pub mod trends_app;
