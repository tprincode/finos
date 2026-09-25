//! Request-scoped lists for Income Plan week walks. Load once; reuse across weeks.

use std::collections::HashMap;

use uuid::Uuid;

use crate::contracts::{
    AccountRecord, BasisGetBody, DividendGetBody, IssuerDeclarationRecord, LotRecord,
    PlanHistoryRecord, PositionCharacteristicRecord, SecurityRecord,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;

pub(crate) struct IncomePlanLists {
    pub accounts: Vec<AccountRecord>,
    pub securities: Vec<SecurityRecord>,
    pub dividend: DividendGetBody,
    pub plans: Vec<PlanHistoryRecord>,
    pub characteristics: Vec<PositionCharacteristicRecord>,
    pub basis: BasisGetBody,
    pub activities: Vec<crate::contracts::ActivityRecord>,
    pub plan_versions: Vec<PlanHistoryRecord>,
    pub last_update_by_sec: HashMap<Uuid, Option<String>>,
}

pub(crate) type DeclDateCache = HashMap<(Uuid, String), Vec<String>>;
pub(crate) type DeclRowCache = HashMap<Uuid, Vec<IssuerDeclarationRecord>>;

pub(crate) async fn load_income_plan_lists(
    canonical: &dyn Canonical,
) -> Result<IncomePlanLists, PlatformError> {
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let dividend = canonical.dividend_get().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let activities = canonical.activity_list().await.unwrap_or_default();
    let plan_versions = canonical
        .plan_history_version_list()
        .await
        .unwrap_or_else(|_| plans.clone());
    let last_update_by_sec = match canonical.collector_set().await {
        Ok(set) => set
            .items
            .into_iter()
            .map(|item| {
                (
                    item.security_id,
                    financial_domain::income_plan::last_update_success(
                        item.last_run_ok,
                        &item.last_run_at,
                    ),
                )
            })
            .collect(),
        Err(_) => HashMap::new(),
    };
    Ok(IncomePlanLists {
        accounts,
        securities,
        dividend,
        plans,
        characteristics,
        basis,
        activities,
        plan_versions,
        last_update_by_sec,
    })
}

pub(crate) fn lots_open_as_of<'a>(
    lots: &'a [LotRecord],
    week_end: &str,
) -> HashMap<Uuid, Vec<&'a LotRecord>> {
    let mut lots_by_security: HashMap<Uuid, Vec<&LotRecord>> = HashMap::new();
    for lot in lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let opened = lot.opened_on.get(..10).unwrap_or(lot.opened_on.as_str());
        if opened > week_end {
            continue;
        }
        lots_by_security
            .entry(lot.security_id)
            .or_default()
            .push(lot);
    }
    lots_by_security
}
