//! FinanceClient port — local Tauri IPC and remote HTTP share this contract (ADR-0006).

use crate::contracts::{
    CommandRequest, CommandResult, QueryRequest, QueryResult, FINANCE_CLIENT_CONTRACT_VERSION,
};

/// Versioned command/query transport. V1: LocalTauriFinanceClient. Later: RemoteHttpFinanceClient.
pub trait FinanceClient: Send + Sync {
    fn contract_version(&self) -> &'static str {
        FINANCE_CLIENT_CONTRACT_VERSION
    }

    fn execute_command(&self, request: CommandRequest) -> CommandResult;

    fn execute_query(&self, request: QueryRequest) -> QueryResult;
}
