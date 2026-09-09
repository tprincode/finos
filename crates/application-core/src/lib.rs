//! Application services layer.
//!
//! Use cases, workflows, transactions, approvals and orchestration.
//! Coordinates domain components through ports; contains no UI logic.

pub mod contracts;
pub mod core_functions;
pub mod data_snapshot;
pub mod golden;
pub mod handoff;
pub mod income_plan_display;
pub mod ports;
pub mod production_seed;
pub mod queries;
pub mod trends_app;
