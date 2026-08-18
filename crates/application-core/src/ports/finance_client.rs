//! FinanceClient port — local Tauri IPC and remote HTTP share this contract (ADR-0006).

/// Marker trait for versioned command/query transport.
pub trait FinanceClient: Send + Sync {
    fn contract_version(&self) -> &'static str {
        "1.0.0-draft"
    }
}
