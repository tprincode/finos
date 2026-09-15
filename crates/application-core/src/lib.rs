//! Application services layer.
//!
//! Use cases, workflows, transactions, approvals and orchestration.
//! Coordinates domain components through ports; contains no UI logic.

pub mod account_value;
pub mod cart;
pub mod cash_management;
pub mod cash_pile;
pub mod contracts;
pub mod core_functions;
pub mod data_snapshot;
pub mod dividend_plan;
pub mod golden;
pub mod handoff;
pub mod income_plan_display;
pub mod last_price_window;
pub mod ports;
pub mod production_seed;
pub mod queries;
pub mod trends_app;
