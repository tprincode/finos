//! Versioned FinanceClient envelopes and DTOs (ADR-0006).
//! TypeScript mirror: packages/app-contracts.

use financial_domain::money::Money;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FINANCE_CLIENT_CONTRACT_VERSION: &str = "1.0.0-draft";

/// Schema version including distribution characterization (Wave 4).
pub const SCHEMA_VERSION: &str = "15";
/// Marketplace MAGI 2026.1 after owner-approved oracles.
pub const CALCULATION_VERSION: &str = "magi-2026.1";
pub const APP_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub contract_version: String,
    pub command_name: String,
    pub correlation_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub contract_version: String,
    pub query_name: String,
    pub correlation_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub contract_version: String,
    pub command_name: String,
    pub correlation_id: Uuid,
    pub ok: bool,
    pub error_code: Option<String>,
    pub body_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub contract_version: String,
    pub query_name: String,
    pub correlation_id: Uuid,
    pub ok: bool,
    pub error_code: Option<String>,
    pub body_json: Option<String>,
}

/// Snapshot identity on the wire (ADR-0007, V1.1 §7). Matches snapshot-service::SnapshotManifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotIdentity {
    pub database_id: Uuid,
    pub snapshot_id: Uuid,
    pub parent_snapshot_id: Option<Uuid>,
    pub device_id: Uuid,
    pub device_name: String,
    pub change_sequence: u64,
    pub last_event_at: String,
    pub schema_version: String,
    pub calculation_version: String,
    pub app_version: String,
    pub database_hash: String,
    pub evidence_manifest_hash: String,
    pub validation_status: String,
    pub restore_test_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfig {
    pub device_id: Uuid,
    pub device_name: String,
    pub database_id: Uuid,
    pub schema_version: String,
    pub calculation_version: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffDecision {
    OpenNormally,
    BlockUntilRestore,
    AllowWritePublishPending,
    BranchConflict,
    UnverifiedHandoff,
    RejectInvalidKeepLast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffStatusBody {
    pub decision: HandoffDecision,
    pub writes_allowed: bool,
    pub message: String,
    pub local_head: Option<SnapshotIdentity>,
    pub published_head: Option<SnapshotIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionState {
    Safe,
    Watch,
    LikelyOver,
    Over,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCompleteness {
    Complete,
    Incomplete,
    PendingReview,
}

/// MAGI output shape (V1.1 §11.4). Calculated by financial-domain via storage-sqlite.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MagiProjection {
    pub applicable_threshold: Money,
    pub actual_included_ytd: Money,
    pub known_remaining: Money,
    pub base_forecast: Money,
    pub conservative_forecast: Money,
    pub uncertain_amount: Money,
    pub raw_headroom: Money,
    pub protected_headroom: Money,
    pub data_completeness: DataCompleteness,
    pub decision_state: DecisionState,
    pub warnings: Vec<String>,
    pub calculation_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MagiTaxPaymentBody {
    pub amount_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorPlanBody {
    pub plan_id: Uuid,
    pub version: u32,
    pub remaining_minor: i64,
    pub scale: u8,
    pub approved_on: String,
    pub supersedes_plan_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BurndownBody {
    pub cash_minor: i64,
    pub obligation_minor: i64,
    pub surplus_minor: i64,
    pub sufficient: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AllocationTargetRecord {
    pub target_id: Uuid,
    pub name: String,
    pub target_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AllocationGetBody {
    pub targets: Vec<AllocationTargetRecord>,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CartItemRecord {
    pub item_id: Uuid,
    pub symbol: String,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CartGetBody {
    pub items: Vec<CartItemRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRunRecord {
    pub run_id: Uuid,
    pub scenario: String,
    pub hypothetical_pnl_minor: i64,
    pub scale: u8,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BacktestGetBody {
    pub runs: Vec<BacktestRunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationReviewRecord {
    pub review_id: Uuid,
    pub fact_key: String,
    pub classification: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationReviewGetBody {
    pub reviews: Vec<ClassificationReviewRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiRunRecord {
    pub run_id: Uuid,
    pub prompt: String,
    pub recommendation: String,
    pub provider: String,
    pub model: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiRunListBody {
    pub runs: Vec<AiRunRecord>,
}

fn default_row_version() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub account_id: Uuid,
    pub name: String,
    pub kind: String,
    #[serde(default = "default_row_version")]
    pub row_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityRecord {
    pub security_id: Uuid,
    pub symbol: String,
    pub name: String,
    #[serde(default)]
    pub crf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRecord {
    pub evidence_id: Uuid,
    pub content_hash: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub account_name: String,
    pub symbol: Option<String>,
    pub activity_type: String,
    pub amount_minor: Option<i64>,
    pub scale: u8,
    pub occurred_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchRecord {
    pub batch_id: Uuid,
    pub source_id: String,
    pub content_hash: String,
    pub status: String,
    pub candidate_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRecord {
    pub activity_id: Uuid,
    pub account_id: Uuid,
    pub security_id: Option<Uuid>,
    pub activity_type: String,
    pub amount_minor: i64,
    pub scale: u8,
    pub occurred_on: String,
    pub corrects_activity_id: Option<Uuid>,
    pub import_batch_id: Option<Uuid>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub audit_id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionRecord {
    pub exception_id: Uuid,
    pub code: String,
    pub message: String,
    pub acknowledged: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWeekBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileCounts {
    pub accounts: u64,
    pub securities: u64,
    pub evidence: u64,
    pub import_batches: u64,
    pub posted_activities: u64,
    pub amount_minor_sum: i64,
    pub scale: u8,
    pub audit_records: u64,
    pub exceptions_open: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendDeclaration {
    pub declaration_id: Uuid,
    pub security_symbol: String,
    pub declared_on: String,
    pub amount_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendActual {
    pub actual_id: Uuid,
    pub account_id: Uuid,
    pub security_id: Option<Uuid>,
    pub occurred_on: String,
    pub amount_minor: i64,
    pub scale: u8,
    pub activity_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendGetBody {
    pub actuals: Vec<DividendActual>,
    pub declarations: Vec<DividendDeclaration>,
    pub actual_total_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanBody {
    pub planned_minor: i64,
    pub actual_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBody {
    pub actual_dividend_minor: i64,
    pub planned_income_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub occurred_on: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsBody {
    pub points: Vec<TrendPoint>,
    pub total_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotRecord {
    pub lot_id: Uuid,
    pub account_id: Uuid,
    pub security_id: Uuid,
    pub opened_on: String,
    pub origin: String,
    pub quantity_minor: i64,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub performance_basis_minor: i64,
    pub tax_basis_minor: i64,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub scale: u8,
    pub crf_zero_cost: bool,
    pub opening_activity_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotAssignmentRecord {
    pub assignment_id: Uuid,
    pub lot_id: Uuid,
    pub activity_id: Uuid,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
    pub proceeds_minor: i64,
    pub performance_cost_minor: i64,
    pub tax_cost_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BasisGetBody {
    pub lots: Vec<LotRecord>,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoiBody {
    pub proceeds_minor: i64,
    pub performance_cost_minor: i64,
    pub tax_cost_minor: i64,
    pub performance_gain_minor: i64,
    pub tax_gain_minor: i64,
    pub dividend_actual_minor: i64,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrokerLotReconcileBody {
    pub sold_quantity_minor: i64,
    pub assigned_quantity_minor: i64,
    pub unmatched_sells: u64,
    pub matched: bool,
    pub quantity_scale: u8,
}

/// Reporting rollup over open lots (ADR-0008). Not authoritative vs `lot`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionLineBody {
    pub account_id: Uuid,
    pub account_name: String,
    pub security_id: Uuid,
    pub symbol: String,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub lot_count: u64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionDetailsBody {
    pub positions: Vec<PositionLineBody>,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub scale: u8,
}

/// Decision-support view over MAGI (ADR-0008). Not a posted MAGI fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaxProjectionBody {
    pub source_query: String,
    pub decision_state: DecisionState,
    pub actual_included_ytd: Money,
    pub applicable_threshold: Money,
    pub data_completeness: DataCompleteness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotRecommendBody {
    pub lot_ids: Vec<Uuid>,
}

/// Check-for-update status. Fail closed: never applied, never posted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterCheckBody {
    pub applied: bool,
    pub posted: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedAccount {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedSecurity {
    pub symbol: String,
    pub name: String,
    pub crf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedPlan {
    pub symbol: String,
    pub amount_per_share_minor: i64,
    pub amount_scale: u8,
    pub planning_periods_per_year: u8,
    pub effective_from: String,
    pub decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedCharacteristic {
    pub symbol: String,
    pub payment_frequency: String,
    pub risk_tier: String,
    pub provider: String,
    pub underlying: String,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_pct_2026_estimate_minor: Option<i64>,
    pub roc_pct_2026_actual_minor: Option<i64>,
    pub roc_pct_2024_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub div_type: String,
    pub needs_roc_research: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedLot {
    pub account_name: String,
    pub symbol: String,
    pub opened_on: String,
    pub origin: String,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
    pub performance_basis_minor: i64,
    pub tax_basis_minor: i64,
    pub scale: u8,
    pub is_open: bool,
    pub row_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedYieldBatch {
    pub source_id: String,
    pub filename: String,
    pub content: String,
    pub candidates: Vec<ImportCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedDisbursement {
    pub account_name: String,
    pub activity_type: String,
    pub amount_minor: i64,
    pub scale: u8,
    pub occurred_on: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedDocument {
    pub accounts: Vec<ProductionSeedAccount>,
    pub securities: Vec<ProductionSeedSecurity>,
    pub lots: Vec<ProductionSeedLot>,
    pub yield_batches: Vec<ProductionSeedYieldBatch>,
    pub disbursements: Vec<ProductionSeedDisbursement>,
    #[serde(default)]
    pub plans: Vec<ProductionSeedPlan>,
    #[serde(default)]
    pub characteristics: Vec<ProductionSeedCharacteristic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSeedLoadBody {
    pub already_loaded: bool,
    pub account_count: u64,
    pub security_count: u64,
    pub lot_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanLineBody {
    pub account_name: String,
    pub actual_minor: i64,
    pub planned_minor: i64,
    pub plan_known: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanDrillBody {
    pub account_name: String,
    pub symbol: String,
    pub occurred_on: String,
    pub amount_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanWeekBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
    pub status: String,
    pub lines: Vec<IncomePlanLineBody>,
    pub drilldown: Vec<IncomePlanDrillBody>,
    pub latest_actual_on: Option<String>,
    pub yield_count: u64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBurndownLineBody {
    pub account_name: String,
    pub inflow_minor: i64,
    pub outflow_minor: i64,
    pub floor_known: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBurndownBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
    pub status: String,
    pub note: String,
    pub lines: Vec<DashboardBurndownLineBody>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsLotBody {
    pub lot_id: Uuid,
    pub account_name: String,
    pub symbol: String,
    pub opened_on: String,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsGetBody {
    pub lots: Vec<HoldingsLotBody>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdSummaryBody {
    pub account_count: u64,
    pub open_lot_count: u64,
    pub yield_count: u64,
    pub disbursement_count: u64,
    pub latest_yield_on: Option<String>,
    pub plan_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanHistoryRecord {
    pub plan_history_id: Uuid,
    pub security_id: Uuid,
    pub amount_per_share_minor: i64,
    pub amount_scale: u8,
    pub planning_periods_per_year: u8,
    pub effective_from: String,
    pub decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionCharacteristicRecord {
    pub security_id: Uuid,
    pub payment_frequency: String,
    pub risk_tier: String,
    pub provider: String,
    pub underlying: String,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_pct_2026_estimate_minor: Option<i64>,
    pub roc_pct_2026_actual_minor: Option<i64>,
    pub roc_pct_2024_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub div_type: String,
    pub needs_roc_research: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorRowBody {
    pub symbol: String,
    pub payment_frequency: String,
    pub plan_known: bool,
    pub plan_per_share_minor: i64,
    pub plan_scale: u8,
    pub planning_periods_per_year: u8,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub plan_payment_minor: i64,
    pub remaining_performance_minor: i64,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalculatorGetBody {
    pub rows: Vec<CalculatorRowBody>,
    pub plan_count: u64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistributionRecord {
    pub characterization_id: Uuid,
    pub activity_id: Uuid,
    pub category: String,
    pub amount_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistributionGetBody {
    pub characterizations: Vec<DistributionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IssuerDeclarationRecord {
    pub declaration_id: Uuid,
    pub security_id: Uuid,
    pub amount_per_share_minor: Option<i64>,
    pub amount_scale: u8,
    pub payment_period: String,
    pub source: String,
    pub entered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceQuoteBody {
    pub price_quote_id: Uuid,
    pub security_id: Uuid,
    pub price_minor: i64,
    pub scale: u8,
    pub as_of_at: String,
    pub source: String,
    pub validation_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPriceBody {
    pub security_id: Uuid,
    pub price_minor: Option<i64>,
    pub scale: u8,
    pub freshness: String,
    pub price_derived_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalTemplateRecord {
    pub security_id: Uuid,
    pub price_source: String,
    pub source_symbol: String,
    pub declaration_source: String,
    pub lookback_count: u8,
    pub payment_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanReviewBody {
    pub security_id: Uuid,
    pub observation_count: u64,
    pub most_current_minor: Option<i64>,
    pub avg6_minor: Option<i64>,
    pub min_minor: Option<i64>,
    pub max_minor: Option<i64>,
    pub average_minor: Option<i64>,
    pub eighty_pct_of_avg_minor: Option<i64>,
    pub avg6_complete: bool,
    pub full_analysis_possible: bool,
    pub confirm_blocked: bool,
    pub incomplete_reason_required: bool,
    pub amount_scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentLotBody {
    pub lot_id: Uuid,
    pub account_name: String,
    pub opened_on: String,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentGetBody {
    pub security_id: Uuid,
    pub symbol: String,
    pub name: String,
    pub payment_frequency: String,
    pub risk_tier: String,
    pub provider: String,
    pub underlying: String,
    pub plan_known: bool,
    pub plan_per_share_minor: i64,
    pub plan_scale: u8,
    pub planning_periods_per_year: u8,
    pub plan_reason: String,
    pub plan_effective_from: String,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub remaining_performance_minor: i64,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub price: CurrentPriceBody,
    pub review: PlanReviewBody,
    pub template: Option<RetrievalTemplateRecord>,
    pub lots: Vec<InvestmentLotBody>,
    pub declaration_count: u64,
    pub first_lot_complete: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceRetrievalSetBody {
    pub security_ids: Vec<Uuid>,
}

