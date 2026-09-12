//! Versioned FinanceClient envelopes and DTOs (ADR-0006).
//! TypeScript mirror: packages/app-contracts.

use financial_domain::money::Money;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FINANCE_CLIENT_CONTRACT_VERSION: &str = "1.0.0-draft";

/// Must equal the latest SQLite migration id (currently 0028_issuer_declaration_dedupe).
pub const SCHEMA_VERSION: &str = "28";
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
    #[serde(default)]
    pub candidate_id: Option<Uuid>,
    /// ready | duplicate | blocked — filled by ImportBatchGet, empty when staging.
    #[serde(default)]
    pub validation: String,
    #[serde(default)]
    pub issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchRecord {
    pub batch_id: Uuid,
    pub source_id: String,
    pub content_hash: String,
    pub status: String,
    pub candidate_count: u64,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub posted_count: u64,
    #[serde(default)]
    pub skipped_duplicate_count: u64,
    #[serde(default)]
    pub error_count: u64,
    #[serde(default)]
    pub process_lines: Vec<String>,
    #[serde(default)]
    pub candidates: Vec<ImportCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportPendingBody {
    pub batch: Option<ImportBatchRecord>,
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
    #[serde(default)]
    pub federal_withholding_minor: i64,
    #[serde(default)]
    pub state_withholding_minor: i64,
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
pub struct WorkTicketRecord {
    pub ticket_id: Uuid,
    pub security_id: Uuid,
    pub symbol: String,
    pub field: String,
    pub code: String,
    pub tool: String,
    pub reason: String,
    pub urls_tried: String,
    pub opened_on: String,
    pub last_seen_on: String,
    pub status: String,
    #[serde(default)]
    pub filed_on: String,
    #[serde(default)]
    pub completed_how: String,
    #[serde(default)]
    pub owner_note: String,
    #[serde(default)]
    pub retrieve_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkTicketListBody {
    pub items: Vec<WorkTicketRecord>,
    pub open_count: u64,
    pub symbol_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkTicketSyncMissesBody {
    pub scanned: u64,
    pub raised: u64,
    pub open_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWeekBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
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
    #[serde(default)]
    pub already_posted: bool,
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
pub struct CashDividendCoverageRow {
    pub account_id: Uuid,
    pub account_name: String,
    pub security_id: Uuid,
    pub symbol: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashDividendCoverageBody {
    pub as_of_date: String,
    pub month: String,
    pub missing_count: u64,
    pub raised_count: u64,
    pub acknowledged_count: u64,
    pub positions: Vec<CashDividendCoverageRow>,
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

/// One Sat–Fri Trends week: entered sources + derived chart fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsWeekSourceRecord {
    pub period_end: String,
    #[serde(default)]
    pub period_start: String,
    pub profit_minor: i64,
    pub monthly_divs_minor: i64,
    pub fidelity_total_minor: i64,
    pub schwab_total_minor: i64,
    pub income_cash_minor: i64,
    pub acct9_cash_minor: i64,
    pub acct9_etf_value_minor: i64,
    pub scale: u8,
    pub captured_at: String,
    #[serde(default)]
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsWeekCaptureBody {
    pub period_start: String,
    pub period_end: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
    pub captured_at: String,
    pub closed: bool,
    pub exists: bool,
    pub prior: Option<TrendsWeekSourceRecord>,
    pub current: Option<TrendsWeekSourceRecord>,
    pub car_balance_minor: Option<i64>,
    pub income_balance_minor: Option<i64>,
    pub health_balance_minor: Option<i64>,
    pub roth_balance_minor: Option<i64>,
    pub speculation_balance_minor: Option<i64>,
    /// Suggested Profit from Investment Activity Ledger (dividends + known realized types).
    pub suggested_profit_minor: i64,
    /// Suggested Monthly DIVS from Income Plan planned_minor.
    pub suggested_monthly_divs_minor: i64,
    /// Suggested Acct9 70% from open lots (T7 interim classification).
    pub suggested_acct9_etf_proxy_minor: Option<i64>,
    pub missing_required: Vec<String>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsDistributionLine {
    pub activity_type: String,
    pub account_name: String,
    pub amount_minor: i64,
    pub occurred_on: String,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsDistributionBody {
    pub gross_minor: i64,
    pub lines: Vec<TrendsDistributionLine>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsTaxMonitorBody {
    pub federal_withholding_minor: i64,
    pub projected_liability_minor: Option<i64>,
    pub gap_minor: Option<i64>,
    pub warning: bool,
    pub aca_threshold_minor: Option<i64>,
    pub aca_coverage_year: Option<i32>,
    pub note: String,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsOverviewBody {
    pub fid_sch_combined_minor: Option<i64>,
    pub wk_to_wk_change_minor: Option<i64>,
    pub profit_minor: Option<i64>,
    pub monthly_divs_minor: Option<i64>,
    pub div_delta_minor: Option<i64>,
    pub total_cash_minor: Option<i64>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsBody {
    /// Dividend actuals (M3 agreement with DividendGet / DashboardGet).
    pub points: Vec<TrendPoint>,
    pub total_minor: i64,
    /// Weekly Trends tab series for charts.
    #[serde(default)]
    pub weeks: Vec<TrendsWeekPoint>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub overview: Option<TrendsOverviewBody>,
    #[serde(default)]
    pub distributions: Option<TrendsDistributionBody>,
    #[serde(default)]
    pub tax_monitor: Option<TrendsTaxMonitorBody>,
    #[serde(default)]
    pub missing_required: Vec<String>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendsWeekPoint {
    pub period_end: String,
    #[serde(default)]
    pub period_start: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
    pub profit_minor: i64,
    pub monthly_divs_minor: i64,
    pub div_delta_minor: i64,
    pub fidelity_total_minor: i64,
    pub schwab_total_minor: i64,
    pub fid_sch_combined_minor: i64,
    pub wk_to_wk_change_minor: i64,
    pub income_cash_minor: i64,
    pub acct9_cash_minor: i64,
    pub acct9_etf_proxy_minor: i64,
    pub total_cash_minor: i64,
    pub car_balance_minor: Option<i64>,
    pub income_balance_minor: Option<i64>,
    pub health_balance_minor: Option<i64>,
    pub roth_balance_minor: Option<i64>,
    pub speculation_balance_minor: Option<i64>,
    #[serde(default)]
    pub closed: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceSnapshotRecord {
    pub snapshot_id: Uuid,
    pub account_id: Uuid,
    pub period_end: String,
    pub balance_minor: i64,
    pub scale: u8,
    pub captured_at: String,
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
    #[serde(default)]
    pub market_value_minor: Option<i64>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountPositionTotalBody {
    pub account_id: Uuid,
    pub account_name: String,
    pub symbol_count: u64,
    pub open_lot_count: u64,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub market_value_minor: Option<i64>,
    pub scale: u8,
}

/// One stored day of holdings market value for an account (or the Fidelity rollup).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountMarketValueDailyRecord {
    pub snapshot_id: String,
    pub account_id: String,
    pub account_name: String,
    pub as_of: String,
    pub market_value_minor: Option<i64>,
    pub market_value_complete: bool,
    pub scale: u8,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountValuePointBody {
    pub as_of: String,
    pub market_value_minor: Option<i64>,
    pub market_value_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountValueSeriesBody {
    pub account_id: String,
    pub account_name: String,
    pub custodian: String,
    pub current_minor: Option<i64>,
    pub current_complete: bool,
    pub points: Vec<AccountValuePointBody>,
    #[serde(default)]
    pub trends_points: Vec<AccountValuePointBody>,
    pub scale: u8,
}

impl Default for AccountValueSeriesBody {
    fn default() -> Self {
        Self {
            account_id: financial_domain::account_value::SCHWAB_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::SCHWAB_TOTAL_NAME.to_string(),
            custodian: "Schwab".into(),
            current_minor: None,
            current_complete: false,
            points: Vec::new(),
            trends_points: Vec::new(),
            scale: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RiskSymbolValueBody {
    pub symbol: String,
    pub risk_tier: String,
    pub market_value_minor: Option<i64>,
    pub market_value_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RiskGroupValueBody {
    pub risk_tier: String,
    pub current_minor: Option<i64>,
    pub current_complete: bool,
    pub symbols: Vec<RiskSymbolValueBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RiskValuePointBody {
    pub as_of: String,
    pub foundation_minor: Option<i64>,
    pub core_minor: Option<i64>,
    pub risk_on_minor: Option<i64>,
    pub undecided_minor: Option<i64>,
    pub total_minor: Option<i64>,
    pub market_value_complete: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RiskValueHomeBody {
    pub current_total_minor: Option<i64>,
    pub current_complete: bool,
    pub groups: Vec<RiskGroupValueBody>,
    pub points: Vec<RiskValuePointBody>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountValueHomeBody {
    pub as_of: String,
    pub accounts: Vec<AccountValueSeriesBody>,
    pub fidelity: AccountValueSeriesBody,
    #[serde(default)]
    pub schwab: AccountValueSeriesBody,
    #[serde(default)]
    pub risk: RiskValueHomeBody,
    pub note: String,
    pub scale: u8,
}

/// Individual importable workbooks written under raw-data/<as_of>/.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataSnapshotExportBody {
    pub as_of: String,
    pub folder: String,
    pub files: Vec<String>,
    pub account_count: u64,
    pub lot_count: u64,
    pub yield_count: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionDetailsBody {
    pub positions: Vec<PositionLineBody>,
    pub account_totals: Vec<AccountPositionTotalBody>,
    pub symbol_count: u64,
    pub account_count: u64,
    pub open_lot_count: u64,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub market_value_minor: Option<i64>,
    pub market_value_complete: bool,
    #[serde(default = "default_true")]
    pub qty_reconcile_ok: bool,
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
    #[serde(default = "default_true")]
    pub is_active: bool,
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
pub struct ProductionSeedTrendsWeek {
    pub period_end: String,
    pub profit_minor: i64,
    pub monthly_divs_minor: i64,
    pub fidelity_total_minor: i64,
    pub schwab_total_minor: i64,
    pub income_cash_minor: i64,
    pub acct9_cash_minor: i64,
    pub acct9_etf_value_minor: i64,
    pub car_balance_minor: Option<i64>,
    pub income_balance_minor: Option<i64>,
    pub health_balance_minor: Option<i64>,
    pub roth_balance_minor: Option<i64>,
    pub speculation_balance_minor: Option<i64>,
    pub scale: u8,
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
    #[serde(default)]
    pub trends_weeks: Vec<ProductionSeedTrendsWeek>,
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
pub struct ProviderDeclarationSourcesApplyBody {
    pub updated: u64,
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
pub struct IncomePlanPositionAccountBody {
    pub account_name: String,
    pub actual_minor: i64,
    #[serde(default)]
    pub actual_known: bool,
    pub planned_minor: i64,
    pub plan_known: bool,
    #[serde(default)]
    pub declaration_minor: i64,
    #[serde(default)]
    pub declaration_known: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanPositionBody {
    pub symbol: String,
    #[serde(default)]
    pub cadence: String,
    #[serde(default)]
    pub pay_on: String,
    pub actual_minor: i64,
    #[serde(default)]
    pub actual_known: bool,
    pub planned_minor: i64,
    pub plan_known: bool,
    #[serde(default)]
    pub declaration_minor: i64,
    #[serde(default)]
    pub declaration_known: bool,
    #[serde(default)]
    pub declaration_per_share_minor: Option<i64>,
    #[serde(default)]
    pub declaration_per_share_scale: u8,
    #[serde(default)]
    pub declaration_entered_on: Option<String>,
    #[serde(default)]
    pub declaration_current: bool,
    pub scale: u8,
    #[serde(default)]
    pub accounts: Vec<IncomePlanPositionAccountBody>,
    /// Last successful declaration-collector run date. Failed or never-run is null.
    #[serde(default)]
    pub last_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanWeekBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
    pub status: String,
    pub lines: Vec<IncomePlanLineBody>,
    pub positions: Vec<IncomePlanPositionBody>,
    pub drilldown: Vec<IncomePlanDrillBody>,
    pub latest_actual_on: Option<String>,
    pub yield_count: u64,
    pub scale: u8,
    #[serde(default)]
    pub miss_count: u64,
    #[serde(default)]
    pub amount_exception_count: u64,
    #[serde(default)]
    pub variance_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanGridWeekMeta {
    pub start: String,
    pub end: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanMoneyCell {
    pub week_end: String,
    pub amount_minor: Option<i64>,
    #[serde(default)]
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanTable1Row {
    pub id: String,
    pub label: String,
    pub year_minor: Option<i64>,
    #[serde(default)]
    pub year_pct_minor: Option<i64>,
    pub cells: Vec<IncomePlanMoneyCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanTable2Row {
    pub symbol: String,
    pub cadence: String,
    pub last_update: Option<String>,
    #[serde(default)]
    pub declaration_per_share_minor: Option<i64>,
    #[serde(default)]
    pub declaration_per_share_scale: u8,
    #[serde(default)]
    pub declaration_entered_on: Option<String>,
    #[serde(default)]
    pub declaration_current: bool,
    pub cells: Vec<IncomePlanMoneyCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanTable2Group {
    pub cadence: String,
    pub rows: Vec<IncomePlanTable2Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanGridBody {
    pub as_of_date: String,
    pub historical_weeks: u32,
    pub future_weeks: u32,
    pub selected_accounts: Vec<String>,
    pub table1_columns: Vec<String>,
    pub weeks: Vec<IncomePlanGridWeekMeta>,
    pub table1: Vec<IncomePlanTable1Row>,
    pub table2: Vec<IncomePlanTable2Group>,
    pub selected_week_end: String,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanExportBody {
    pub format: String,
    pub destination_kind: String,
    pub default_file_name: String,
    pub landscape: bool,
    pub bytes_base64: String,
    pub print_html: String,
    pub cover: IncomePlanCoverBody,
    #[serde(default)]
    pub sheet_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IncomePlanCoverBody {
    pub pattern: String,
    pub printed_at: String,
    pub selected_accounts: Vec<String>,
    pub historical_weeks: u32,
    pub future_weeks: u32,
    pub week_ending: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendPerformancePositionBody {
    pub symbol: String,
    pub actual_minor: i64,
    #[serde(default)]
    pub actual_known: bool,
    pub planned_minor: i64,
    pub plan_known: bool,
    #[serde(default)]
    pub declaration_minor: i64,
    #[serde(default)]
    pub declaration_known: bool,
    pub pct_of_plan_minor: Option<i64>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendPerformanceWeekBody {
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
    pub actual_minor: i64,
    #[serde(default)]
    pub actual_known: bool,
    pub planned_minor: i64,
    pub plan_known: bool,
    #[serde(default)]
    pub declaration_minor: i64,
    #[serde(default)]
    pub declaration_known: bool,
    pub pct_of_plan_minor: Option<i64>,
    pub positions: Vec<DividendPerformancePositionBody>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendPerformanceSummaryBody {
    pub actual_minor: i64,
    pub planned_minor: i64,
    pub plan_known: bool,
    pub pct_of_plan_minor: Option<i64>,
    pub avg_weekly_actual_minor: Option<i64>,
    pub avg_weekly_plan_minor: Option<i64>,
    pub week_count: u32,
    pub known_plan_week_count: u32,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DividendPerformanceBody {
    pub as_of_date: String,
    pub range: String,
    pub range_start: Option<String>,
    pub this_week_start: String,
    pub weeks: Vec<DividendPerformanceWeekBody>,
    pub summary: DividendPerformanceSummaryBody,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBurndownLineBody {
    pub account_name: String,
    pub inflow_minor: i64,
    pub outflow_minor: i64,
    pub floor_known: bool,
    /// Latest Trends `AccountBalanceSnapshot` for this control account, if any.
    #[serde(default)]
    pub ending_balance_minor: Option<i64>,
    #[serde(default)]
    pub ending_balance_known: bool,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBurndownBody {
    pub as_of_date: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub week_year: i32,
    #[serde(default)]
    pub week_number: u8,
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
pub struct DataSummaryBody {
    pub account_count: u64,
    pub open_lot_count: u64,
    pub yield_count: u64,
    pub disbursement_count: u64,
    pub latest_yield_on: Option<String>,
    pub plan_count: u64,
    pub symbol_count: u64,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
    pub last_price_count: u64,
    #[serde(default)]
    pub last_price_refreshed_on: Option<String>,
    /// Enabled collectors whose daily issuer retrieve is current for `declaration_as_of`.
    #[serde(default)]
    pub declaration_count: u64,
    /// Run-enabled collectors (open lot + assigned source). One expected retrieve each today.
    #[serde(default)]
    pub declaration_collector_count: u64,
    #[serde(default)]
    pub declaration_refreshed_on: Option<String>,
    /// Local calendar day of the standing DeclarationRefresh.
    #[serde(default)]
    pub declaration_as_of: String,
    /// Open work_ticket rows. Owner work queue — not today's miss count.
    #[serde(default)]
    pub open_ticket_count: u64,
    pub market_value_minor: Option<i64>,
    pub market_value_complete: bool,
    /// Lifetime paid dividends (all yield actuals).
    pub income_earned_minor: i64,
    pub scale: u8,
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
    #[serde(default)]
    pub effective_to: String,
    pub decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LookthroughHolding {
    pub ticker: String,
    #[serde(default)]
    pub weight_bps: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LookthroughSectorWeight {
    pub label: String,
    #[serde(default)]
    pub weight_bps: Option<i64>,
}

/// Owner-confirmed look-through research. Not a full holdings book and not PositionExposure.
/// Blank / unknown status is not 0% and does not invent a sector type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LookthroughResearch {
    #[serde(default)]
    pub theme_strategy: String,
    #[serde(default)]
    pub primary_risk_driver: String,
    #[serde(default = "unknown_concentration")]
    pub concentration_status: String,
    #[serde(default)]
    pub top_holdings: Vec<LookthroughHolding>,
    #[serde(default)]
    pub sector_weights: Vec<LookthroughSectorWeight>,
    #[serde(default)]
    pub concentration_as_of: Option<String>,
    #[serde(default)]
    pub vol_proxy: String,
    #[serde(default)]
    pub tax_character: String,
    #[serde(default)]
    pub risk_tier_suggestion: String,
    #[serde(default)]
    pub risk_tier_suggestion_reason: String,
    #[serde(default)]
    pub covered_call: bool,
    #[serde(default)]
    pub leveraged: bool,
}

fn unknown_concentration() -> String {
    "unknown".into()
}

impl Default for LookthroughResearch {
    fn default() -> Self {
        Self {
            theme_strategy: String::new(),
            primary_risk_driver: String::new(),
            concentration_status: unknown_concentration(),
            top_holdings: Vec::new(),
            sector_weights: Vec::new(),
            concentration_as_of: None,
            vol_proxy: String::new(),
            tax_character: String::new(),
            risk_tier_suggestion: String::new(),
            risk_tier_suggestion_reason: String::new(),
            covered_call: false,
            leveraged: false,
        }
    }
}

impl LookthroughResearch {
    pub fn from_json_str(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "{}" {
            return Self::default();
        }
        serde_json::from_str(trimmed).unwrap_or_default()
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
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
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub lookthrough: LookthroughResearch,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedPaymentPattern {
    pub security_id: Uuid,
    pub declaration_weekday: String,
    pub exdate_weekday: String,
    pub payday_weekday: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionTaxProfile {
    pub security_id: Uuid,
    pub expected_handling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionMasterRowBody {
    pub security_id: Uuid,
    pub symbol: String,
    pub name: String,
    pub risk_tier: String,
    pub provider: String,
    pub underlying: String,
    pub payment_frequency: String,
    pub div_type: String,
    pub needs_roc_research: bool,
    pub is_active: bool,
    pub notes: String,
    pub tax_handling: String,
    pub declaration_weekday: String,
    pub exdate_weekday: String,
    pub payday_weekday: String,
    pub remaining_quantity_minor: i64,
    pub quantity_scale: u8,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub unit_cost_minor: Option<i64>,
    pub last_price_minor: Option<i64>,
    pub last_price_scale: Option<u8>,
    pub price_freshness: String,
    pub price_derived_valid: bool,
    pub market_value_minor: Option<i64>,
    pub allocation_bps: Option<i64>,
    pub plan_known: bool,
    pub plan_per_share_minor: i64,
    pub plan_scale: u8,
    pub annual_plan_minor: Option<i64>,
    pub plan_yoc_bps: Option<i64>,
    pub plan_fwd_yield_bps: Option<i64>,
    pub most_current_fwd_yield_bps: Option<i64>,
    pub unrealized_pnl_bps: Option<i64>,
    pub roc_pct_2024_actual_minor: Option<i64>,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_pct_2026_estimate_minor: Option<i64>,
    pub roc_pct_2026_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub declaration_count: u64,
    pub period_dated: bool,
    pub bear_price_return_bps: Option<i64>,
    pub bear_total_return_bps: Option<i64>,
    pub bull_total_return_bps: Option<i64>,
    pub completeness: String,
    #[serde(default)]
    pub total_distributions_received_minor: Option<i64>,
    #[serde(default)]
    pub roc_distributions_minor: Option<i64>,
    #[serde(default)]
    pub cost_recovery_bps: Option<i64>,
    #[serde(default)]
    pub distributions_scope: String,
    #[serde(default)]
    pub evidence: Option<EvidenceDimensionsBody>,
    #[serde(default)]
    pub bear_cushion_bps: Option<i64>,
    #[serde(default)]
    pub bull_price_return_bps: Option<i64>,
    #[serde(default)]
    pub bull_cushion_bps: Option<i64>,
    #[serde(default)]
    pub car_market_value_minor: Option<i64>,
    #[serde(default)]
    pub car_share_of_symbol_bps: Option<i64>,
    #[serde(default)]
    pub car_share_of_data_bps: Option<i64>,
    #[serde(default)]
    pub roc_research_status: String,
    #[serde(default)]
    pub declaration_freshness: String,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionMasterGetBody {
    pub rows: Vec<PositionMasterRowBody>,
    pub data_market_value_minor: Option<i64>,
    pub market_value_complete: bool,
    pub scale: u8,
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
    pub roc_pct_2026_estimate_minor: Option<i64>,
    pub roc_pct_2026_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    #[serde(default)]
    pub last_price_minor: Option<i64>,
    #[serde(default)]
    pub last_price_scale: Option<u8>,
    #[serde(default)]
    pub price_freshness: String,
    #[serde(default)]
    pub market_value_minor: Option<i64>,
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
pub struct CashManagementWeekRow {
    pub activity_id: Uuid,
    pub account_id: Uuid,
    pub account_name: String,
    pub activity_type: String,
    pub occurred_on: String,
    pub gross_minor: i64,
    pub federal_withholding_minor: i64,
    pub state_withholding_minor: i64,
    pub net_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementWeekBody {
    pub period_start: String,
    pub period_end: String,
    pub rows: Vec<CashManagementWeekRow>,
    pub week_gross_minor: i64,
    pub week_withholding_minor: i64,
    pub week_net_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementSaturdayDraft {
    pub open: bool,
    pub activity_type: String,
    pub suggested_account_id: Option<Uuid>,
    pub suggested_account_name: String,
    pub occurred_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementSsaRecent {
    pub occurred_on: String,
    pub amount_minor: i64,
    pub account_name: String,
    pub extra_audit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementTomSsa {
    pub year_month: String,
    pub expected_minor: i64,
    pub label: String,
    pub status: String,
    pub posted_minor: Option<i64>,
    pub extra_audit: bool,
    pub suggested_account_id: Option<Uuid>,
    pub suggested_account_name: String,
    pub recent: Vec<CashManagementSsaRecent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementRemindersBody {
    pub as_of_date: String,
    pub saturday_draft: CashManagementSaturdayDraft,
    pub tom_ssa: CashManagementTomSsa,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementMonthRow {
    pub account_id: Uuid,
    pub account_name: String,
    pub activity_type: String,
    pub count: u32,
    pub gross_minor: i64,
    pub federal_withholding_minor: i64,
    pub state_withholding_minor: i64,
    pub net_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CashManagementMonthBody {
    pub year_month: String,
    pub period_start: String,
    pub period_end: String,
    pub rows: Vec<CashManagementMonthRow>,
    pub month_gross_minor: i64,
    pub month_withholding_minor: i64,
    pub month_net_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationHistoryCellBody {
    pub amount_per_share_minor: Option<i64>,
    pub amount_scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationHistoryRowBody {
    pub symbol: String,
    pub payment_frequency: String,
    pub cells: Vec<DeclarationHistoryCellBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationHistoryGetBody {
    pub as_of_date: String,
    pub start_on: String,
    pub end_on: String,
    pub cadence_filter: String,
    pub week_ends: Vec<String>,
    #[serde(default)]
    pub week_starts: Vec<String>,
    #[serde(default)]
    pub week_years: Vec<i32>,
    #[serde(default)]
    pub week_numbers: Vec<u8>,
    pub rows: Vec<DeclarationHistoryRowBody>,
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
pub struct IssuerPayDateRecord {
    pub pay_date_id: Uuid,
    pub security_id: Uuid,
    pub pay_on: String,
    pub source: String,
    pub recorded_at: String,
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
    #[serde(default)]
    pub as_of_at: Option<String>,
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
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub calendar_policy: String,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub last_run_ok: Option<bool>,
    #[serde(default)]
    pub last_run_message: String,
    #[serde(default)]
    pub last_content_hash: String,
    /// Assigned vendor only collects when true. DIV-1 + disabled stays a loud miss.
    #[serde(default)]
    pub collector_enabled: bool,
    /// First distribution-eligible date (ISO). Optional; only consulted when
    /// a retrieve returns fewer than 12 paid declarations.
    #[serde(default)]
    pub inception_on: String,
    /// Reusable owner/search 19a-1 notice URL. Empty until search hit or owner paste.
    #[serde(default)]
    pub roc_source_url: String,
    /// Phase I history URL tries. 0 unused, 1 first fail (prompt second URL), 2 loud fail.
    #[serde(default)]
    pub history_url_attempts: u8,
}

impl Default for RetrievalTemplateRecord {
    fn default() -> Self {
        Self {
            security_id: Uuid::nil(),
            price_source: String::new(),
            source_symbol: String::new(),
            declaration_source: String::new(),
            lookback_count: 12,
            payment_source: String::new(),
            source_url: String::new(),
            calendar_policy: String::new(),
            last_run_at: String::new(),
            last_run_ok: None,
            last_run_message: String::new(),
            last_content_hash: String::new(),
            collector_enabled: false,
            inception_on: String::new(),
            roc_source_url: String::new(),
            history_url_attempts: 0,
        }
    }
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
pub struct InvestmentDeclarationBody {
    pub payment_period: String,
    pub amount_per_share_minor: Option<i64>,
    pub amount_scale: u8,
    pub source: String,
    #[serde(default)]
    pub entered_at: String,
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
    pub remaining_tax_minor: i64,
    pub roc_pct_2024_actual_minor: Option<i64>,
    pub roc_pct_2025_actual_minor: Option<i64>,
    pub roc_pct_2026_estimate_minor: Option<i64>,
    pub roc_pct_2026_actual_minor: Option<i64>,
    pub roc_scale: Option<u8>,
    pub notes: String,
    pub div_type: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub needs_roc_research: bool,
    #[serde(default)]
    pub tax_handling: String,
    #[serde(default)]
    pub declaration_weekday: String,
    #[serde(default)]
    pub exdate_weekday: String,
    #[serde(default)]
    pub payday_weekday: String,
    #[serde(default)]
    pub unit_cost_minor: Option<i64>,
    #[serde(default)]
    pub plan_fwd_yield_bps: Option<i64>,
    #[serde(default)]
    pub most_current_fwd_yield_bps: Option<i64>,
    #[serde(default)]
    pub unrealized_pnl_bps: Option<i64>,
    pub price: CurrentPriceBody,
    pub review: PlanReviewBody,
    pub template: Option<RetrievalTemplateRecord>,
    pub lots: Vec<InvestmentLotBody>,
    pub declarations: Vec<InvestmentDeclarationBody>,
    pub declaration_count: u64,
    pub first_lot_complete: bool,
    pub market_value_minor: Option<i64>,
    pub unrealized_performance_minor: Option<i64>,
    pub annual_plan_minor: Option<i64>,
    pub plan_yoc_bps: Option<i64>,
    pub most_current_vs_plan_bps: Option<i64>,
    pub periods: Vec<BacktestPeriodRecord>,
    pub results: Vec<PositionBacktestResultBody>,
    pub evidence: Option<EvidenceDimensionsBody>,
    pub suggestion: Option<TierSuggestionBody>,
    #[serde(default)]
    pub total_distributions_received_minor: Option<i64>,
    #[serde(default)]
    pub roc_distributions_minor: Option<i64>,
    #[serde(default)]
    pub cost_recovery_bps: Option<i64>,
    #[serde(default)]
    pub distributions_scope: String,
    #[serde(default)]
    pub car_market_value_minor: Option<i64>,
    #[serde(default)]
    pub car_share_of_symbol_bps: Option<i64>,
    #[serde(default)]
    pub car_share_of_data_bps: Option<i64>,
    #[serde(default)]
    pub roc_research_status: String,
    #[serde(default)]
    pub declaration_freshness: String,
    #[serde(default)]
    pub roc_estimate_method: String,
    #[serde(default)]
    pub roc_estimate_source_url: String,
    #[serde(default)]
    pub roc_estimate_as_of: String,
    #[serde(default)]
    pub roc_estimate_established_how: String,
    #[serde(default)]
    pub roc_research_completed_at: Option<String>,
    #[serde(default)]
    pub lookthrough: LookthroughResearch,
    #[serde(default)]
    pub collector_complete: bool,
    #[serde(default)]
    pub collector_gaps: Vec<String>,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BacktestPeriodRecord {
    pub period_id: Uuid,
    pub kind: String,
    pub name: String,
    pub start_on: String,
    pub end_on: String,
    pub benchmark_symbol: String,
    pub selection_reason: String,
    pub method: String,
    pub status: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionBacktestResultBody {
    pub result_id: Uuid,
    pub security_id: Uuid,
    pub period_id: Uuid,
    pub price_return_bps: Option<i64>,
    pub total_return_bps: Option<i64>,
    pub cushion_bps: Option<i64>,
    pub max_drawdown_bps: Option<i64>,
    pub recovery_ratio_bps: Option<i64>,
    pub recovery_days: Option<i64>,
    pub income_reliability_bps: Option<i64>,
    pub bear_relative_bps: Option<i64>,
    pub downside_capture_bps: Option<i64>,
    pub upside_capture_bps: Option<i64>,
    pub completeness: String,
    pub source: String,
    pub calculated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDimensionsBody {
    pub income_reliability: Option<i64>,
    pub downside_resilience: Option<i64>,
    pub recovery_upside: Option<i64>,
    pub nav_persistence: Option<i64>,
    pub diversification: Option<i64>,
    pub data_confidence: i64,
    pub known_components: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TierSuggestionBody {
    pub suggested_tier: String,
    pub ruleset: String,
    pub reason: String,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RocCandidateBody {
    pub roc_pct_minor: Option<i64>,
    pub scale: u8,
    pub tax_year: String,
    pub source: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub as_of: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub established_how: String,
    #[serde(default)]
    pub owner_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RocResearchObservation {
    pub observation_id: Uuid,
    pub security_id: Uuid,
    pub roc_pct_minor: Option<i64>,
    pub scale: u8,
    pub tax_year: String,
    pub source: String,
    pub source_url: String,
    pub method: String,
    pub as_of: String,
    pub kind: String,
    pub established_how: String,
    pub owner_override: bool,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RocResearchBody {
    pub security_id: Option<Uuid>,
    pub roc_pct_minor: Option<i64>,
    pub scale: u8,
    pub source: String,
    pub complete: bool,
    pub reason: String,
    pub candidates: Vec<RocCandidateBody>,
    pub remaining_periods: Option<i64>,
    pub remaining_total_minor: Option<i64>,
    pub remaining_ordinary_minor: Option<i64>,
    pub remaining_roc_minor: Option<i64>,
    pub magi_eligible: bool,
    #[serde(default)]
    pub system_roc_pct_minor: Option<i64>,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub as_of: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub established_how: String,
    #[serde(default)]
    pub owner_override: bool,
    #[serde(default)]
    pub observations: Vec<RocResearchObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemainingPaymentDateOverride {
    pub override_id: Uuid,
    pub security_id: Uuid,
    pub original_pay_on: String,
    pub pay_on: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemainingPaymentBody {
    pub pay_on: String,
    pub original_pay_on: String,
    pub month: String,
    pub cash_minor: Option<i64>,
    pub this_lot_cash_minor: Option<i64>,
    pub position_after_cash_minor: Option<i64>,
    pub owner_override: bool,
    #[serde(default)]
    pub date_provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemainingMonthBody {
    pub month: String,
    pub cash_minor: Option<i64>,
    pub this_lot_cash_minor: Option<i64>,
    pub position_after_cash_minor: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemainingYearIncomeBody {
    pub security_id: Uuid,
    pub as_of_date: String,
    pub known: bool,
    pub provenance: String,
    pub payment_frequency: String,
    pub latest_declaration_period: Option<String>,
    pub remaining_periods: Option<i64>,
    pub year_to_go_minor: Option<i64>,
    pub this_lot_year_to_go_minor: Option<i64>,
    pub position_after_year_to_go_minor: Option<i64>,
    pub existing_quantity_minor: i64,
    pub this_lot_quantity_minor: Option<i64>,
    pub hypothetical: bool,
    pub plan_known: bool,
    pub payments: Vec<RemainingPaymentBody>,
    pub months: Vec<RemainingMonthBody>,
    #[serde(default)]
    pub orphaned_overrides: Vec<RemainingPaymentDateOverride>,
    #[serde(default)]
    pub calendar_policy: String,
    pub scale: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceRetrievalItem {
    pub security_id: Uuid,
    pub symbol: String,
    #[serde(default)]
    pub price_source: String,
    #[serde(default)]
    pub source_symbol: String,
    #[serde(default)]
    pub declaration_source: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub calendar_policy: String,
    #[serde(default)]
    pub last_content_hash: String,
    #[serde(default)]
    pub div_type: String,
    #[serde(default)]
    pub collector_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriceRetrievalSetBody {
    pub security_ids: Vec<Uuid>,
    #[serde(default)]
    pub items: Vec<PriceRetrievalItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LastPriceRefreshBody {
    pub attempted: u64,
    pub recorded: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationRefreshBody {
    pub attempted: u64,
    pub recorded: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionDetailsCoverageRow {
    pub security_id: Uuid,
    pub symbol: String,
    pub active: bool,
    pub open_lots: bool,
    pub recorded_status: String,
    pub roc_research_status: String,
    pub declaration_freshness: String,
    pub distributions_scope: String,
    #[serde(default)]
    pub declaration_source: String,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub last_run_ok: Option<bool>,
    #[serde(default)]
    pub last_run_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionDetailsCoverageBody {
    pub rows: Vec<PositionDetailsCoverageRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveRunRecord {
    pub run_id: Uuid,
    pub security_id: Uuid,
    pub kind: String,
    pub requested_at: String,
    pub ok: bool,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    pub attempted: u64,
    pub recorded: u64,
    pub skipped: u64,
    pub unchanged: u64,
    /// Full candidate / analytics payload for admin review. Never HTML.
    #[serde(default)]
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorSetItem {
    pub security_id: Uuid,
    pub symbol: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub div_type: String,
    #[serde(default)]
    pub payment_frequency: String,
    #[serde(default)]
    pub declaration_source: String,
    #[serde(default)]
    pub price_source: String,
    #[serde(default)]
    pub source_symbol: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub calendar_policy: String,
    #[serde(default)]
    pub collector_enabled: bool,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub last_run_ok: Option<bool>,
    #[serde(default)]
    pub last_run_message: String,
    #[serde(default)]
    pub last_content_hash: String,
    #[serde(default)]
    pub roc_source_url: String,
    #[serde(default)]
    pub open_lots: bool,
    /// ISO date; empty means full lookback required.
    #[serde(default)]
    pub inception_on: String,
    /// Same `collector_is_complete` LotOpen uses (grandfather is already-has-lots).
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub gaps: Vec<String>,
    #[serde(default)]
    pub fill_gaps_provider_blank: bool,
    #[serde(default)]
    pub fill_gaps_frequency_blank: bool,
    #[serde(default)]
    pub fill_gaps_div_type_blank: bool,
    #[serde(default)]
    pub fill_gaps_roc_blank: bool,
    #[serde(default)]
    pub paid_declaration_count: u8,
    #[serde(default)]
    pub remaining_planned: Option<i64>,
    #[serde(default)]
    pub remaining_expected: Option<u8>,
    #[serde(default)]
    pub successful_run_count: u64,
    #[serde(default)]
    pub failure_count: u64,
    #[serde(default)]
    pub open_ticket_count: u64,
    #[serde(default)]
    pub latest_ticket_field: String,
    #[serde(default)]
    pub last_price_as_of: String,
    #[serde(default)]
    pub last_price_freshness: String,
    #[serde(default)]
    pub underlying: String,
    #[serde(default)]
    pub roc_estimate_minor: Option<i64>,
    #[serde(default)]
    pub roc_scale: u8,
    #[serde(default)]
    pub roc_tax_year: String,
    #[serde(default)]
    pub open_lot_count: u64,
    #[serde(default)]
    pub last_payable_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorSetBody {
    pub items: Vec<CollectorSetItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorStatsBody {
    pub assigned: u64,
    pub enabled: u64,
    pub ran_today: u64,
    /// Enabled fleet whose last retrieve today is not OK (failed or not yet).
    #[serde(default)]
    pub still_miss: u64,
    /// Distinct fleet securities with any ok=0 declaration run today.
    #[serde(default)]
    pub had_miss_today: u64,
    /// Same as still_miss. Kept so older clients do not read today's ledger as "failed now."
    pub miss_today: u64,
    pub unchanged_today: u64,
    pub cash_par: u64,
    pub price_current: u64,
    pub price_stale: u64,
    pub open_exceptions: u64,
    /// Declaration retrieves today that are not income-fleet payers (e.g. MSTU, SOXL, TSLL).
    #[serde(default)]
    pub ran_outside_fleet: Vec<String>,
    pub as_of_date: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionResearchSeedBody {
    pub security_id: Uuid,
    pub symbol: String,
    pub declaration_source: String,
    pub source_url: String,
    pub calendar_policy: String,
    /// Inferred Weekly/Monthly/Quarterly when paid history or issuer label supports 52/12/4.
    #[serde(default)]
    pub payment_frequency: String,
    /// True when the post-seed retrieve path reported ok (misses stay unknown, not $0).
    pub retrieve_ok: bool,
    #[serde(default)]
    pub retrieve_code: String,
    #[serde(default)]
    pub retrieve_message: String,
    /// Proposed current-year 19a-1 estimate (not confirmed, not 1099 actual).
    #[serde(default)]
    pub roc_pct_minor: Option<i64>,
    #[serde(default)]
    pub roc_scale: u8,
    #[serde(default)]
    pub roc_source_url: String,
    #[serde(default)]
    pub roc_method: String,
    #[serde(default)]
    pub roc_kind: String,
    #[serde(default)]
    pub roc_as_of: String,
    #[serde(default)]
    pub roc_established_how: String,
    /// Always false on Process A propose — estimate is not research-complete.
    #[serde(default)]
    pub roc_complete: bool,
    /// 19a-1 URL probes (url + HTTP status) when live retrieve ran.
    #[serde(default)]
    pub roc_probes: Vec<serde_json::Value>,
    #[serde(default)]
    pub needs_second_url: bool,
    #[serde(default)]
    pub second_url_tried: bool,
    #[serde(default)]
    pub adapter_failed: bool,
    #[serde(default)]
    pub paid_count: u8,
    #[serde(default)]
    pub needs_inception_confirm: bool,
    #[serde(default)]
    pub inception_candidate: String,
    #[serde(default)]
    pub expected_paid_since_inception: Option<u8>,
    #[serde(default)]
    pub inception_search_miss: bool,
}

/// Shared hole-fill research path (Process A after seed, Position Details Complete research,
/// Collectors Fill research gaps). Writes only blanks; never wipes lots / PlanHistory / decls.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PositionResearchRefreshBody {
    pub security_id: Uuid,
    pub symbol: String,
    pub declaration_source: String,
    pub source_url: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub underlying: String,
    #[serde(default)]
    pub payment_frequency: String,
    pub retrieve_ok: bool,
    #[serde(default)]
    pub retrieve_code: String,
    #[serde(default)]
    pub retrieve_message: String,
    #[serde(default)]
    pub roc_pct_minor: Option<i64>,
    #[serde(default)]
    pub roc_scale: u8,
    #[serde(default)]
    pub roc_source_url: String,
    #[serde(default)]
    pub roc_method: String,
    #[serde(default)]
    pub roc_kind: String,
    #[serde(default)]
    pub roc_as_of: String,
    #[serde(default)]
    pub roc_established_how: String,
    /// Always false — estimate is not research-complete / not 1099.
    #[serde(default)]
    pub roc_complete: bool,
    #[serde(default)]
    pub roc_probes: Vec<serde_json::Value>,
    #[serde(default)]
    pub needs_roc_research: bool,
    #[serde(default)]
    pub needs_second_url: bool,
    #[serde(default)]
    pub second_url_tried: bool,
    #[serde(default)]
    pub adapter_failed: bool,
    #[serde(default)]
    pub paid_count: u8,
    #[serde(default)]
    pub needs_inception_confirm: bool,
    #[serde(default)]
    pub inception_candidate: String,
    #[serde(default)]
    pub expected_paid_since_inception: Option<u8>,
    #[serde(default)]
    pub inception_search_miss: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorFieldDecisionRecord {
    pub security_id: Uuid,
    pub field: String,
    pub decision: String,
    pub noted_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchGapItem {
    pub security_id: Uuid,
    pub symbol: String,
    #[serde(default)]
    pub provider_blank: bool,
    #[serde(default)]
    pub underlying_blank: bool,
    #[serde(default)]
    pub frequency_blank: bool,
    #[serde(default)]
    pub roc_observation_blank: bool,
    #[serde(default)]
    pub div_type_blank: bool,
    #[serde(default)]
    pub roc_estimate_blank: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchGapsGetBody {
    pub items: Vec<ResearchGapItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorRetrieveBody {
    pub security_id: Uuid,
    pub symbol: String,
    pub run_id: Uuid,
    pub attempted: u64,
    pub recorded: u64,
    pub skipped: u64,
    pub unchanged: u64,
    pub ok: bool,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    /// Echo of persisted retrieve_run.payload_json for the admin symbol page.
    #[serde(default)]
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Div1ComplianceSummaryRow {
    pub security_id: Uuid,
    pub symbol: String,
    pub future_pay_dates_qty: u64,
    #[serde(default)]
    pub future_pay_dates: Vec<String>,
    pub prior_declarations_qty: u64,
    pub current_declaration_amount_minor: Option<i64>,
    pub current_declaration_amount_scale: Option<u8>,
    pub current_declaration_date: String,
    pub last_run_ok: Option<bool>,
    pub required_paid: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Div1ComplianceSummaryBody {
    pub rows: Vec<Div1ComplianceSummaryRow>,
    pub as_of_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveRunListBody {
    pub runs: Vec<RetrieveRunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreFunctionSentinel {
    pub path: String,
    pub must_contain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreFunctionItem {
    pub id: String,
    pub menu_area: String,
    pub function: String,
    pub last_changed: String,
    pub last_verified: String,
    #[serde(default)]
    pub also_verify: Vec<String>,
    pub sentinels: Vec<CoreFunctionSentinel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreFunctionsGetBody {
    pub items: Vec<CoreFunctionItem>,
}

