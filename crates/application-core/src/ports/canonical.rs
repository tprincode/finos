//! Canonical foundation port: registries, capture, ledger, audit, exceptions, week.

use async_trait::async_trait;
use uuid::Uuid;

use crate::contracts::{
    AccountRecord, ActivityRecord, AuditRecord, BasisGetBody, BrokerLotReconcileBody,
    CanonicalWeekBody, DividendActual, DividendDeclaration, DividendGetBody, EvidenceRecord,
    ExceptionRecord, ImportBatchRecord, ImportCandidate, IncomePlanBody, LotAssignmentRecord,
    LotRecommendBody, LotRecord, MagiProjection, MagiTaxPaymentBody, ReconcileCounts, RoiBody,
    SecurityRecord, AllocationGetBody, AiRunListBody, AiRunRecord, BacktestGetBody, BurndownBody,
    CalculatorPlanBody, CartGetBody, ClassificationReviewGetBody, PositionDetailsBody,
    TaxProjectionBody, DistributionGetBody, PlanHistoryRecord, PositionCharacteristicRecord,
    IssuerDeclarationRecord, IssuerPayDateRecord, PriceQuoteBody, CurrentPriceBody,
    RetrievalTemplateRecord, PriceRetrievalSetBody, BacktestPeriodRecord,
    PositionBacktestResultBody,
    RocResearchObservation, RemainingPaymentDateOverride, ExpectedPaymentPattern,
    PositionTaxProfile, RetrieveRunRecord, CollectorSetBody, CollectorStatsBody,
    AccountBalanceSnapshotRecord, TrendsWeekSourceRecord, WorkTicketRecord,
    CollectorFieldDecisionRecord,
};
use crate::ports::platform::PlatformError;

fn ni<T>() -> Result<T, PlatformError> {
    Err(PlatformError::new(
        "not_implemented",
        "canonical adapter not attached",
    ))
}

#[async_trait]
#[allow(unused_variables)]
pub trait Canonical: Send + Sync {
    async fn account_register(
        &self,
        name: String,
        kind: String,
    ) -> Result<AccountRecord, PlatformError> { ni() }
    async fn account_update(
        &self,
        account_id: Uuid,
        name: Option<String>,
        kind: Option<String>,
        expected_version: Option<i64>,
    ) -> Result<AccountRecord, PlatformError> { ni() }
    async fn snapshot_import_sqlite(
        &self,
        sqlite_path: String,
    ) -> Result<ReconcileCounts, PlatformError> { ni() }
    async fn account_get(&self, account_id: Uuid) -> Result<AccountRecord, PlatformError> { ni() }
    async fn account_list(&self) -> Result<Vec<AccountRecord>, PlatformError> { ni() }

    async fn security_register(
        &self,
        symbol: String,
        name: String,
        crf: bool,
    ) -> Result<SecurityRecord, PlatformError> { ni() }
    async fn security_update(
        &self,
        security_id: Uuid,
        name: Option<String>,
        symbol: Option<String>,
    ) -> Result<SecurityRecord, PlatformError> { ni() }
    async fn security_get(&self, security_id: Uuid) -> Result<SecurityRecord, PlatformError> { ni() }
    async fn security_list(&self) -> Result<Vec<SecurityRecord>, PlatformError> { ni() }

    async fn evidence_store(
        &self,
        filename: String,
        content: Vec<u8>,
    ) -> Result<EvidenceRecord, PlatformError> { ni() }
    async fn evidence_get(&self, evidence_id: Uuid) -> Result<EvidenceRecord, PlatformError> { ni() }

    async fn import_stage(
        &self,
        source_id: String,
        filename: String,
        content: Vec<u8>,
        candidates: Vec<ImportCandidate>,
        default_account: Option<String>,
    ) -> Result<ImportBatchRecord, PlatformError> { ni() }
    async fn import_validate(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> { ni() }
    async fn import_approve(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> { ni() }
    async fn import_post(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> { ni() }
    async fn import_batch_get(&self, batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> { ni() }
    async fn import_pending_get(&self) -> Result<Option<ImportBatchRecord>, PlatformError> { ni() }

    async fn activity_post(
        &self,
        account_id: Uuid,
        security_id: Option<Uuid>,
        activity_type: String,
        amount_minor: Option<i64>,
        scale: u8,
        occurred_on: String,
        corrects_activity_id: Option<Uuid>,
        import_batch_id: Option<Uuid>,
        idempotency_key: Option<String>,
    ) -> Result<ActivityRecord, PlatformError> { ni() }
    async fn activity_correct(
        &self,
        activity_id: Uuid,
        amount_minor: Option<i64>,
        scale: u8,
        occurred_on: String,
    ) -> Result<ActivityRecord, PlatformError> { ni() }
    async fn activity_get(&self, activity_id: Uuid) -> Result<ActivityRecord, PlatformError> { ni() }
    async fn activity_list(&self) -> Result<Vec<ActivityRecord>, PlatformError> { ni() }
    async fn activity_reassign_security(
        &self,
        activity_id: Uuid,
        security_id: Uuid,
    ) -> Result<ActivityRecord, PlatformError> { ni() }

    async fn audit_list(&self) -> Result<Vec<AuditRecord>, PlatformError> { ni() }
    async fn exception_acknowledge(
        &self,
        exception_id: Uuid,
    ) -> Result<ExceptionRecord, PlatformError> { ni() }
    async fn exception_list(&self) -> Result<Vec<ExceptionRecord>, PlatformError> { ni() }
    async fn exception_raise(
        &self,
        _code: String,
        _message: String,
    ) -> Result<ExceptionRecord, PlatformError> {
        ni()
    }
    async fn work_ticket_raise(
        &self,
        _record: WorkTicketRecord,
    ) -> Result<WorkTicketRecord, PlatformError> {
        ni()
    }
    async fn work_ticket_list(
        &self,
        _security_id: Option<Uuid>,
        _status: Option<String>,
    ) -> Result<Vec<WorkTicketRecord>, PlatformError> {
        ni()
    }
    async fn work_ticket_get(
        &self,
        _ticket_id: Uuid,
    ) -> Result<WorkTicketRecord, PlatformError> {
        ni()
    }
    async fn work_ticket_update(
        &self,
        _record: WorkTicketRecord,
    ) -> Result<WorkTicketRecord, PlatformError> {
        ni()
    }
    async fn collector_field_decision_set(
        &self,
        _record: CollectorFieldDecisionRecord,
    ) -> Result<CollectorFieldDecisionRecord, PlatformError> {
        ni()
    }
    async fn collector_field_decision_list(
        &self,
        _security_id: Uuid,
    ) -> Result<Vec<CollectorFieldDecisionRecord>, PlatformError> {
        ni()
    }
    async fn canonical_week_get(&self, as_of_date: String) -> Result<CanonicalWeekBody, PlatformError> { ni() }
    async fn reconcile_counts(&self) -> Result<ReconcileCounts, PlatformError> { ni() }

    async fn dividend_declare(
        &self,
        security_symbol: String,
        declared_on: String,
        amount_minor: i64,
        scale: u8,
    ) -> Result<DividendDeclaration, PlatformError> { ni() }
    async fn dividend_actual_record(
        &self,
        account_id: Uuid,
        security_id: Option<Uuid>,
        occurred_on: String,
        amount_minor: Option<i64>,
        scale: u8,
        idempotency_key: Option<String>,
    ) -> Result<DividendActual, PlatformError> { ni() }
    async fn dividend_get(&self) -> Result<DividendGetBody, PlatformError> { ni() }
    async fn income_plan_update(
        &self,
        planned_minor: i64,
        scale: u8,
    ) -> Result<IncomePlanBody, PlatformError> { ni() }
    async fn income_plan_get(&self) -> Result<IncomePlanBody, PlatformError> { ni() }

    async fn lot_open(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        opened_on: String,
        origin: String,
        quantity_minor: i64,
        quantity_scale: u8,
        performance_basis_minor: i64,
        tax_basis_minor: i64,
        scale: u8,
        opening_activity_id: Option<Uuid>,
        is_open: bool,
    ) -> Result<LotRecord, PlatformError> { ni() }
    async fn lot_reassign_security(
        &self,
        lot_id: Uuid,
        security_id: Uuid,
    ) -> Result<LotRecord, PlatformError> { ni() }
    async fn lot_assign(
        &self,
        lot_id: Uuid,
        activity_id: Uuid,
        quantity_minor: i64,
        quantity_scale: u8,
    ) -> Result<LotAssignmentRecord, PlatformError> { ni() }
    async fn lot_get(&self, lot_id: Uuid) -> Result<LotRecord, PlatformError> { ni() }
    async fn basis_get(&self) -> Result<BasisGetBody, PlatformError> { ni() }
    async fn roi_get(&self) -> Result<RoiBody, PlatformError> { ni() }
    async fn lot_recommend(&self, account_id: Uuid, security_id: Uuid) -> Result<LotRecommendBody, PlatformError> { ni() }
    async fn broker_lot_reconcile(&self) -> Result<BrokerLotReconcileBody, PlatformError> { ni() }
    async fn position_details_get(&self) -> Result<PositionDetailsBody, PlatformError> { ni() }
    async fn tax_projection_get(&self) -> Result<TaxProjectionBody, PlatformError> { ni() }

    async fn magi_rule_set(
        &self,
        threshold_minor: i64,
        safety_reserve_minor: i64,
        scale: u8,
    ) -> Result<MagiProjection, PlatformError> { ni() }
    async fn magi_fact_record(
        &self,
        source_id: String,
        treatment: String,
        amount_minor: i64,
        scale: u8,
        category: String,
    ) -> Result<MagiProjection, PlatformError> { ni() }
    async fn magi_coverage_set(
        &self,
        completeness: String,
        remaining_minor: i64,
        withholding_minor: i64,
        form_total_minor: i64,
        warnings: Vec<String>,
    ) -> Result<MagiProjection, PlatformError> { ni() }
    async fn magi_projection_get(&self) -> Result<MagiProjection, PlatformError> { ni() }
    async fn magi_tax_payment_get(&self) -> Result<MagiTaxPaymentBody, PlatformError> { ni() }
    async fn magi_adjustment_record(
        &self,
        adjustment_id: String,
        amount_minor: i64,
        scale: u8,
        status: String,
        reason: String,
    ) -> Result<MagiProjection, PlatformError> { ni() }

    async fn plan_approve(
        &self,
        remaining_minor: i64,
        scale: u8,
        approved_on: String,
    ) -> Result<CalculatorPlanBody, PlatformError> { ni() }
    async fn plan_get(&self) -> Result<CalculatorPlanBody, PlatformError> { ni() }
    async fn burndown_get(&self) -> Result<BurndownBody, PlatformError> { ni() }
    async fn plan_history_record(
        &self,
        security_id: Uuid,
        amount_per_share_minor: i64,
        amount_scale: u8,
        planning_periods_per_year: u8,
        effective_from: String,
        decision_reason: String,
    ) -> Result<PlanHistoryRecord, PlatformError> { ni() }
    async fn plan_history_list(&self) -> Result<Vec<PlanHistoryRecord>, PlatformError> { ni() }
    async fn plan_history_version_list(&self) -> Result<Vec<PlanHistoryRecord>, PlatformError> {
        self.plan_history_list().await
    }
    async fn position_characteristic_upsert(
        &self,
        record: PositionCharacteristicRecord,
    ) -> Result<PositionCharacteristicRecord, PlatformError> { ni() }
    async fn position_characteristic_list(&self) -> Result<Vec<PositionCharacteristicRecord>, PlatformError> { ni() }

    async fn plan_history_confirm(
        &self,
        security_id: Uuid,
        amount_per_share_minor: i64,
        amount_scale: u8,
        planning_periods_per_year: u8,
        effective_from: String,
        decision_reason: String,
    ) -> Result<PlanHistoryRecord, PlatformError> { ni() }
    async fn issuer_declaration_record(
        &self,
        security_id: Uuid,
        amount_per_share_minor: Option<i64>,
        amount_scale: u8,
        payment_period: String,
        source: String,
        entered_at: String,
    ) -> Result<IssuerDeclarationRecord, PlatformError> { ni() }
    /// Kept for tests. Product retrieve never replaces stored paid rows.
    async fn issuer_declaration_replace_paid(
        &self,
        security_id: Uuid,
        amount_per_share_minor: Option<i64>,
        amount_scale: u8,
        payment_period: String,
        source: String,
        entered_at: String,
    ) -> Result<IssuerDeclarationRecord, PlatformError> {
        self.issuer_declaration_record(
            security_id,
            amount_per_share_minor,
            amount_scale,
            payment_period,
            source,
            entered_at,
        )
        .await
    }
    async fn issuer_declaration_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<IssuerDeclarationRecord>, PlatformError> { ni() }
    /// Retire an unoccurred placeholder so the pay date can move. Occurred rows stay.
    async fn issuer_declaration_supersede_period(
        &self,
        _security_id: Uuid,
        _payment_period: String,
    ) -> Result<u64, PlatformError> {
        Ok(0)
    }
    async fn issuer_pay_date_replace(
        &self,
        security_id: Uuid,
        as_of: String,
        dates: Vec<IssuerPayDateRecord>,
    ) -> Result<Vec<IssuerPayDateRecord>, PlatformError> { ni() }
    async fn issuer_pay_date_insert(
        &self,
        record: IssuerPayDateRecord,
    ) -> Result<IssuerPayDateRecord, PlatformError> {
        ni()
    }
    async fn issuer_pay_date_supersede_one(
        &self,
        security_id: Uuid,
        pay_on: String,
    ) -> Result<(), PlatformError> {
        ni()
    }
    async fn issuer_pay_date_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<IssuerPayDateRecord>, PlatformError> { ni() }
    async fn issuer_pay_date_dedupe(
        &self,
        _security_id: Uuid,
    ) -> Result<u64, PlatformError> {
        Ok(0)
    }
    async fn price_quote_record(
        &self,
        security_id: Uuid,
        price_minor: i64,
        scale: u8,
        as_of_at: String,
        source: String,
    ) -> Result<PriceQuoteBody, PlatformError> { ni() }
    async fn price_quote_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<PriceQuoteBody>, PlatformError> { ni() }
    async fn price_quote_reject(
        &self,
        price_quote_id: Uuid,
    ) -> Result<PriceQuoteBody, PlatformError> { ni() }
    async fn manual_price_override(
        &self,
        security_id: Uuid,
        price_minor: i64,
        scale: u8,
        reason: String,
        effective_from: String,
    ) -> Result<CurrentPriceBody, PlatformError> { ni() }
    async fn current_price_get(
        &self,
        security_id: Uuid,
        as_of_date: String,
    ) -> Result<CurrentPriceBody, PlatformError> { ni() }
    async fn retrieval_template_set(
        &self,
        record: RetrievalTemplateRecord,
    ) -> Result<RetrievalTemplateRecord, PlatformError> { ni() }
    async fn retrieval_template_get(
        &self,
        security_id: Uuid,
    ) -> Result<Option<RetrievalTemplateRecord>, PlatformError> { ni() }
    async fn retrieval_template_touch_run(
        &self,
        security_id: Uuid,
        ok: bool,
        message: String,
        ran_at: String,
        content_hash: String,
        source_url: &str,
    ) -> Result<(), PlatformError> { ni() }
    async fn price_retrieval_set(&self) -> Result<PriceRetrievalSetBody, PlatformError> { ni() }
    async fn retrieve_run_record(
        &self,
        record: RetrieveRunRecord,
    ) -> Result<RetrieveRunRecord, PlatformError> {
        ni()
    }
    async fn retrieve_run_list(
        &self,
        security_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<RetrieveRunRecord>, PlatformError> {
        ni()
    }
    async fn collector_set(&self) -> Result<CollectorSetBody, PlatformError> { ni() }
    async fn collector_stats(&self, as_of_date: String) -> Result<CollectorStatsBody, PlatformError> {
        ni()
    }

    async fn backtest_period_record(
        &self,
        record: BacktestPeriodRecord,
    ) -> Result<BacktestPeriodRecord, PlatformError> { ni() }
    async fn backtest_period_list(&self) -> Result<Vec<BacktestPeriodRecord>, PlatformError> { ni() }
    async fn backtest_period_get(
        &self,
        period_id: Uuid,
    ) -> Result<BacktestPeriodRecord, PlatformError> { ni() }
    async fn position_backtest_result_record(
        &self,
        record: PositionBacktestResultBody,
    ) -> Result<PositionBacktestResultBody, PlatformError> { ni() }
    async fn position_backtest_result_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<PositionBacktestResultBody>, PlatformError> { ni() }

    async fn roc_observation_record(
        &self,
        record: RocResearchObservation,
    ) -> Result<RocResearchObservation, PlatformError> { ni() }
    async fn roc_observation_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<RocResearchObservation>, PlatformError> { ni() }

    async fn remaining_payment_date_override_record(
        &self,
        record: RemainingPaymentDateOverride,
    ) -> Result<RemainingPaymentDateOverride, PlatformError> { ni() }
    async fn remaining_payment_date_override_list(
        &self,
        security_id: Uuid,
    ) -> Result<Vec<RemainingPaymentDateOverride>, PlatformError> { ni() }

    async fn expected_payment_pattern_upsert(
        &self,
        record: ExpectedPaymentPattern,
    ) -> Result<ExpectedPaymentPattern, PlatformError> { ni() }
    async fn expected_payment_pattern_list(
        &self,
    ) -> Result<Vec<ExpectedPaymentPattern>, PlatformError> { ni() }
    async fn position_tax_profile_upsert(
        &self,
        record: PositionTaxProfile,
    ) -> Result<PositionTaxProfile, PlatformError> { ni() }
    async fn position_tax_profile_list(&self) -> Result<Vec<PositionTaxProfile>, PlatformError> {
        ni()
    }

    async fn allocation_target_set(
        &self,
        name: String,
        target_minor: i64,
        scale: u8,
    ) -> Result<AllocationGetBody, PlatformError> { ni() }
    async fn allocation_get(&self) -> Result<AllocationGetBody, PlatformError> { ni() }

    async fn cart_item_add(
        &self,
        symbol: String,
        quantity_minor: i64,
        quantity_scale: u8,
    ) -> Result<CartGetBody, PlatformError> { ni() }
    async fn cart_item_remove(&self, item_id: Uuid) -> Result<CartGetBody, PlatformError> { ni() }
    async fn cart_get(&self) -> Result<CartGetBody, PlatformError> { ni() }

    async fn backtest_run(
        &self,
        scenario: String,
        hypothetical_pnl_minor: i64,
        scale: u8,
        completed_at: String,
    ) -> Result<BacktestGetBody, PlatformError> { ni() }
    async fn backtest_get(&self) -> Result<BacktestGetBody, PlatformError> { ni() }

    async fn classification_review_record(
        &self,
        fact_key: String,
        classification: String,
        status: String,
    ) -> Result<ClassificationReviewGetBody, PlatformError> { ni() }
    async fn classification_review_get(&self) -> Result<ClassificationReviewGetBody, PlatformError> { ni() }

    async fn distribution_characterize(
        &self,
        activity_id: Uuid,
        category: String,
        amount_minor: i64,
        scale: u8,
    ) -> Result<DistributionGetBody, PlatformError> { ni() }
    async fn distribution_get(&self) -> Result<DistributionGetBody, PlatformError> { ni() }

    async fn ai_analyze(&self, prompt: String) -> Result<AiRunRecord, PlatformError> { ni() }
    async fn ai_run_get(&self, run_id: Uuid) -> Result<AiRunRecord, PlatformError> { ni() }
    async fn analysis_run_list(&self) -> Result<AiRunListBody, PlatformError> { ni() }

    async fn trends_week_upsert(
        &self,
        record: TrendsWeekSourceRecord,
    ) -> Result<TrendsWeekSourceRecord, PlatformError> { ni() }
    async fn trends_week_list(&self) -> Result<Vec<TrendsWeekSourceRecord>, PlatformError> { ni() }
    async fn trends_week_get(
        &self,
        period_end: String,
    ) -> Result<Option<TrendsWeekSourceRecord>, PlatformError> { ni() }
    async fn trends_week_set_closed(
        &self,
        period_end: String,
        closed: bool,
    ) -> Result<(), PlatformError> { ni() }
    async fn trends_series_clear(&self) -> Result<(), PlatformError> { ni() }
    async fn aca_threshold_get(
        &self,
        coverage_year: i32,
        household_size: i32,
        location_code: String,
    ) -> Result<Option<(i64, u8)>, PlatformError> { ni() }
    async fn account_balance_snapshot_upsert(
        &self,
        account_id: Uuid,
        period_end: String,
        balance_minor: i64,
        scale: u8,
        captured_at: String,
    ) -> Result<AccountBalanceSnapshotRecord, PlatformError> { ni() }
    async fn account_balance_snapshot_list(
        &self,
    ) -> Result<Vec<AccountBalanceSnapshotRecord>, PlatformError> { ni() }
    async fn account_market_value_daily_upsert(
        &self,
        record: crate::contracts::AccountMarketValueDailyRecord,
    ) -> Result<crate::contracts::AccountMarketValueDailyRecord, PlatformError> { ni() }
    async fn account_market_value_daily_list(
        &self,
    ) -> Result<Vec<crate::contracts::AccountMarketValueDailyRecord>, PlatformError> { ni() }
}

/// Test double: every method returns not_implemented.
pub struct UnimplementedCanonical;

#[async_trait]
impl Canonical for UnimplementedCanonical {
    async fn account_register(
        &self,
        _name: String,
        _kind: String,
    ) -> Result<AccountRecord, PlatformError> {
        ni()
    }
    async fn account_update(
        &self,
        _account_id: Uuid,
        _name: Option<String>,
        _kind: Option<String>,
        _expected_version: Option<i64>,
    ) -> Result<AccountRecord, PlatformError> {
        ni()
    }
    async fn account_get(&self, _account_id: Uuid) -> Result<AccountRecord, PlatformError> {
        ni()
    }
    async fn account_list(&self) -> Result<Vec<AccountRecord>, PlatformError> {
        ni()
    }
    async fn security_register(
        &self,
        _symbol: String,
        _name: String,
        _crf: bool,
    ) -> Result<SecurityRecord, PlatformError> {
        ni()
    }
    async fn security_update(
        &self,
        _security_id: Uuid,
        _name: Option<String>,
        _symbol: Option<String>,
    ) -> Result<SecurityRecord, PlatformError> {
        ni()
    }
    async fn security_get(&self, _security_id: Uuid) -> Result<SecurityRecord, PlatformError> {
        ni()
    }
    async fn security_list(&self) -> Result<Vec<SecurityRecord>, PlatformError> {
        ni()
    }
    async fn evidence_store(
        &self,
        _filename: String,
        _content: Vec<u8>,
    ) -> Result<EvidenceRecord, PlatformError> {
        ni()
    }
    async fn evidence_get(&self, _evidence_id: Uuid) -> Result<EvidenceRecord, PlatformError> {
        ni()
    }
    async fn import_stage(
        &self,
        _source_id: String,
        _filename: String,
        _content: Vec<u8>,
        _candidates: Vec<ImportCandidate>,
        _default_account: Option<String>,
    ) -> Result<ImportBatchRecord, PlatformError> {
        ni()
    }
    async fn import_validate(&self, _batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        ni()
    }
    async fn import_approve(&self, _batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        ni()
    }
    async fn import_post(&self, _batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        ni()
    }
    async fn import_batch_get(&self, _batch_id: Uuid) -> Result<ImportBatchRecord, PlatformError> {
        ni()
    }
    async fn activity_post(
        &self,
        _account_id: Uuid,
        _security_id: Option<Uuid>,
        _activity_type: String,
        _amount_minor: Option<i64>,
        _scale: u8,
        _occurred_on: String,
        _corrects_activity_id: Option<Uuid>,
        _import_batch_id: Option<Uuid>,
        _idempotency_key: Option<String>,
    ) -> Result<ActivityRecord, PlatformError> {
        ni()
    }
    async fn activity_correct(
        &self,
        _activity_id: Uuid,
        _amount_minor: Option<i64>,
        _scale: u8,
        _occurred_on: String,
    ) -> Result<ActivityRecord, PlatformError> {
        ni()
    }
    async fn activity_get(&self, _activity_id: Uuid) -> Result<ActivityRecord, PlatformError> {
        ni()
    }
    async fn activity_list(&self) -> Result<Vec<ActivityRecord>, PlatformError> {
        ni()
    }
    async fn audit_list(&self) -> Result<Vec<AuditRecord>, PlatformError> {
        ni()
    }
    async fn exception_acknowledge(
        &self,
        _exception_id: Uuid,
    ) -> Result<ExceptionRecord, PlatformError> {
        ni()
    }
    async fn exception_list(&self) -> Result<Vec<ExceptionRecord>, PlatformError> {
        ni()
    }
    async fn canonical_week_get(
        &self,
        _as_of_date: String,
    ) -> Result<CanonicalWeekBody, PlatformError> {
        ni()
    }
    async fn reconcile_counts(&self) -> Result<ReconcileCounts, PlatformError> {
        ni()
    }
    async fn dividend_declare(
        &self,
        _security_symbol: String,
        _declared_on: String,
        _amount_minor: i64,
        _scale: u8,
    ) -> Result<DividendDeclaration, PlatformError> {
        ni()
    }
    async fn dividend_actual_record(
        &self,
        _account_id: Uuid,
        _security_id: Option<Uuid>,
        _occurred_on: String,
        _amount_minor: Option<i64>,
        _scale: u8,
        _idempotency_key: Option<String>,
    ) -> Result<DividendActual, PlatformError> {
        ni()
    }
    async fn dividend_get(&self) -> Result<DividendGetBody, PlatformError> {
        ni()
    }
    async fn income_plan_update(
        &self,
        _planned_minor: i64,
        _scale: u8,
    ) -> Result<IncomePlanBody, PlatformError> {
        ni()
    }
    async fn income_plan_get(&self) -> Result<IncomePlanBody, PlatformError> {
        ni()
    }
    async fn lot_open(
        &self,
        _account_id: Uuid,
        _security_id: Uuid,
        _opened_on: String,
        _origin: String,
        _quantity_minor: i64,
        _quantity_scale: u8,
        _performance_basis_minor: i64,
        _tax_basis_minor: i64,
        _scale: u8,
        _opening_activity_id: Option<Uuid>,
        _is_open: bool,
    ) -> Result<LotRecord, PlatformError> {
        ni()
    }
    async fn lot_assign(
        &self,
        _lot_id: Uuid,
        _activity_id: Uuid,
        _quantity_minor: i64,
        _quantity_scale: u8,
    ) -> Result<LotAssignmentRecord, PlatformError> {
        ni()
    }
    async fn lot_get(&self, _lot_id: Uuid) -> Result<LotRecord, PlatformError> {
        ni()
    }
    async fn basis_get(&self) -> Result<BasisGetBody, PlatformError> {
        ni()
    }
    async fn roi_get(&self) -> Result<RoiBody, PlatformError> {
        ni()
    }
    async fn lot_recommend(
        &self,
        _account_id: Uuid,
        _security_id: Uuid,
    ) -> Result<LotRecommendBody, PlatformError> {
        ni()
    }
    async fn broker_lot_reconcile(&self) -> Result<BrokerLotReconcileBody, PlatformError> {
        ni()
    }
    async fn position_details_get(&self) -> Result<PositionDetailsBody, PlatformError> {
        ni()
    }
    async fn tax_projection_get(&self) -> Result<TaxProjectionBody, PlatformError> {
        ni()
    }
    async fn magi_rule_set(
        &self,
        _threshold_minor: i64,
        _safety_reserve_minor: i64,
        _scale: u8,
    ) -> Result<MagiProjection, PlatformError> {
        ni()
    }
    async fn magi_fact_record(
        &self,
        _source_id: String,
        _treatment: String,
        _amount_minor: i64,
        _scale: u8,
        _category: String,
    ) -> Result<MagiProjection, PlatformError> {
        ni()
    }
    async fn magi_coverage_set(
        &self,
        _completeness: String,
        _remaining_minor: i64,
        _withholding_minor: i64,
        _form_total_minor: i64,
        _warnings: Vec<String>,
    ) -> Result<MagiProjection, PlatformError> {
        ni()
    }
    async fn magi_projection_get(&self) -> Result<MagiProjection, PlatformError> {
        ni()
    }
    async fn magi_tax_payment_get(&self) -> Result<MagiTaxPaymentBody, PlatformError> {
        ni()
    }
    async fn magi_adjustment_record(
        &self,
        _adjustment_id: String,
        _amount_minor: i64,
        _scale: u8,
        _status: String,
        _reason: String,
    ) -> Result<MagiProjection, PlatformError> {
        ni()
    }
    async fn plan_approve(
        &self,
        _remaining_minor: i64,
        _scale: u8,
        _approved_on: String,
    ) -> Result<CalculatorPlanBody, PlatformError> {
        ni()
    }
    async fn plan_get(&self) -> Result<CalculatorPlanBody, PlatformError> {
        ni()
    }
    async fn burndown_get(&self) -> Result<BurndownBody, PlatformError> {
        ni()
    }
    async fn plan_history_record(
        &self,
        _security_id: Uuid,
        _amount_per_share_minor: i64,
        _amount_scale: u8,
        _planning_periods_per_year: u8,
        _effective_from: String,
        _decision_reason: String,
    ) -> Result<PlanHistoryRecord, PlatformError> {
        ni()
    }
    async fn plan_history_list(&self) -> Result<Vec<PlanHistoryRecord>, PlatformError> {
        ni()
    }
    async fn position_characteristic_upsert(
        &self,
        _record: PositionCharacteristicRecord,
    ) -> Result<PositionCharacteristicRecord, PlatformError> {
        ni()
    }
    async fn position_characteristic_list(&self) -> Result<Vec<PositionCharacteristicRecord>, PlatformError> {
        ni()
    }
    async fn allocation_target_set(
        &self,
        _name: String,
        _target_minor: i64,
        _scale: u8,
    ) -> Result<AllocationGetBody, PlatformError> {
        ni()
    }
    async fn allocation_get(&self) -> Result<AllocationGetBody, PlatformError> {
        ni()
    }
    async fn cart_item_add(
        &self,
        _symbol: String,
        _quantity_minor: i64,
        _quantity_scale: u8,
    ) -> Result<CartGetBody, PlatformError> {
        ni()
    }
    async fn cart_item_remove(&self, _item_id: Uuid) -> Result<CartGetBody, PlatformError> {
        ni()
    }
    async fn cart_get(&self) -> Result<CartGetBody, PlatformError> {
        ni()
    }
    async fn backtest_run(
        &self,
        _scenario: String,
        _hypothetical_pnl_minor: i64,
        _scale: u8,
        _completed_at: String,
    ) -> Result<BacktestGetBody, PlatformError> {
        ni()
    }
    async fn backtest_get(&self) -> Result<BacktestGetBody, PlatformError> {
        ni()
    }
    async fn classification_review_record(
        &self,
        _fact_key: String,
        _classification: String,
        _status: String,
    ) -> Result<ClassificationReviewGetBody, PlatformError> {
        ni()
    }
    async fn classification_review_get(&self) -> Result<ClassificationReviewGetBody, PlatformError> {
        ni()
    }
    async fn ai_analyze(&self, _prompt: String) -> Result<AiRunRecord, PlatformError> {
        ni()
    }
    async fn ai_run_get(&self, _run_id: Uuid) -> Result<AiRunRecord, PlatformError> {
        ni()
    }
    async fn analysis_run_list(&self) -> Result<AiRunListBody, PlatformError> {
        ni()
    }

    async fn trends_week_upsert(
        &self,
        _record: TrendsWeekSourceRecord,
    ) -> Result<TrendsWeekSourceRecord, PlatformError> {
        ni()
    }
    async fn trends_week_list(&self) -> Result<Vec<TrendsWeekSourceRecord>, PlatformError> {
        ni()
    }
    async fn trends_week_get(
        &self,
        _period_end: String,
    ) -> Result<Option<TrendsWeekSourceRecord>, PlatformError> {
        ni()
    }
    async fn trends_week_set_closed(
        &self,
        _period_end: String,
        _closed: bool,
    ) -> Result<(), PlatformError> {
        ni()
    }
    async fn trends_series_clear(&self) -> Result<(), PlatformError> {
        ni()
    }
    async fn aca_threshold_get(
        &self,
        _coverage_year: i32,
        _household_size: i32,
        _location_code: String,
    ) -> Result<Option<(i64, u8)>, PlatformError> {
        ni()
    }
    async fn account_balance_snapshot_upsert(
        &self,
        _account_id: Uuid,
        _period_end: String,
        _balance_minor: i64,
        _scale: u8,
        _captured_at: String,
    ) -> Result<AccountBalanceSnapshotRecord, PlatformError> {
        ni()
    }
    async fn account_balance_snapshot_list(
        &self,
    ) -> Result<Vec<AccountBalanceSnapshotRecord>, PlatformError> {
        ni()
    }
}
