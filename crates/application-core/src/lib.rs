//! Application services layer.
//!
//! Use cases, workflows, transactions, approvals and orchestration.
//! Coordinates domain components through ports; contains no UI logic.

pub mod account_value;
pub mod cart;
pub mod cash_coverage;
pub mod cash_management;
pub mod cash_pile;
pub mod cash_register;
pub mod cash_ytd;
pub mod contracts;
pub mod tax_planning;
pub mod core_functions;
pub mod data_snapshot;
pub mod dividend_plan;
pub mod external_register;
pub mod golden;
pub mod handoff;
pub mod income_plan_display;
pub mod last_price_window;
pub mod plan_horizon;
pub mod ports;
pub mod production_seed;
pub mod query_cache;
pub mod queries;
pub mod trends_app;
pub mod week_ahead;
