//! Domain errors. Unknown values stay visible; they are never silent zeros.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("amount is unknown; refusing silent conversion to zero")]
    UnknownAmount,
    #[error("duplicate posting is not allowed")]
    DuplicatePost,
    #[error("lot assignment is explicit; the domain does not assume FIFO")]
    FifoNotAssumed,
    #[error("zero-cost DRIP lots are allowed only on CRF (including FI Roth)")]
    ZeroCostDripNotCrf,
    #[error("lot quantity is insufficient or invalid")]
    InsufficientLotQuantity,
    #[error("lot origin is not recognized")]
    InvalidLotOrigin,
    #[error("money scale mismatch between performance and tax basis")]
    ScaleMismatch,
    #[error("plan confirm is blocked until at least one declaration observation exists")]
    PlanConfirmBlocked,
    #[error("incomplete analysis requires an explicit reason before plan confirm")]
    IncompleteAnalysisRequired,
    #[error("declaration source is required")]
    MissingDeclarationSource,
    #[error("price must be positive; zero is not a quote")]
    NonpositivePrice,
    #[error("backtest period is incomplete; owner must supply dates, kind, method, and reason")]
    RegimePeriodIncomplete,
    #[error("account quantities do not reconcile to open lots")]
    QtyReconcileMismatch,
    #[error("cost recovery cannot use a ROC-reduced economic-cost denominator")]
    CostRecoveryRocReducedDenominator,
    #[error("cash distribution type is not allowed on this post")]
    CashDistributionType,
    #[error("net must equal gross minus withholding and cannot be negative")]
    CashDistributionIdentity,
    #[error("Roth distributions cannot have withholding")]
    RothWithholdingNotAllowed,
}
