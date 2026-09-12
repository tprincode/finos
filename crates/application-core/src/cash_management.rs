//! Cash Management week board, Saturday draft, and Tom SSA confirm.

use crate::contracts::{
    ActivityRecord, CashManagementMonthBody, CashManagementMonthRow,
    CashManagementRemindersBody, CashManagementSaturdayDraft, CashManagementSsaRecent,
    CashManagementTomSsa, CashManagementWeekBody, CashManagementWeekRow,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::Datelike;

pub async fn cash_distribution_post(
    canonical: &dyn Canonical,
    account_id: uuid::Uuid,
    activity_type: String,
    occurred_on: String,
    gross_minor: Option<i64>,
    federal_withholding_minor: i64,
    state_withholding_minor: i64,
    scale: u8,
    idempotency_key: Option<String>,
) -> Result<ActivityRecord, PlatformError> {
    let net = financial_domain::cash_management::validate_cash_distribution(
        &activity_type,
        gross_minor,
        federal_withholding_minor,
        state_withholding_minor,
    )
    .map_err(|err| {
        let code = match err {
            financial_domain::error::DomainError::UnknownAmount => "unknown_amount",
            financial_domain::error::DomainError::CashDistributionType => "cash_distribution_type",
            financial_domain::error::DomainError::CashDistributionIdentity => {
                "cash_distribution_identity"
            }
            financial_domain::error::DomainError::RothWithholdingNotAllowed => {
                "roth_withholding_not_allowed"
            }
            _ => "domain_error",
        };
        PlatformError::new(code, err.to_string())
    })?;
    let _ = net;
    let posted = canonical
        .activity_post(
            account_id,
            None,
            activity_type,
            gross_minor,
            scale,
            occurred_on,
            None,
            None,
            idempotency_key,
        )
        .await?;
    canonical
        .activity_withholding_set(
            posted.activity_id,
            federal_withholding_minor,
            state_withholding_minor,
        )
        .await
}

pub async fn cash_management_week(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CashManagementWeekBody, PlatformError> {
    let as_of = financial_domain::trends::parse_iso_date(as_of).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid asOfDate {as_of}"))
    })?;
    let week = financial_domain::trends::trends_period_for_capture(as_of);
    let period_start = week.start.format("%Y-%m-%d").to_string();
    let period_end = week.end.format("%Y-%m-%d").to_string();
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let superseded: std::collections::HashSet<_> = activities
        .iter()
        .filter_map(|a| a.corrects_activity_id)
        .collect();
    let mut rows = Vec::new();
    for a in &activities {
        if superseded.contains(&a.activity_id) {
            continue;
        }
        if !financial_domain::trends::is_non_roi_distribution(&a.activity_type) {
            continue;
        }
        if !financial_domain::income_plan::occurred_in_week(
            &a.occurred_on,
            &period_start,
            &period_end,
        ) {
            continue;
        }
        let account_name = accounts
            .iter()
            .find(|acct| acct.account_id == a.account_id)
            .map(|acct| acct.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let net = financial_domain::cash_management::net_minor(
            a.amount_minor,
            a.federal_withholding_minor,
            a.state_withholding_minor,
        );
        rows.push(CashManagementWeekRow {
            activity_id: a.activity_id,
            account_id: a.account_id,
            account_name,
            activity_type: a.activity_type.clone(),
            occurred_on: a.occurred_on.clone(),
            gross_minor: a.amount_minor,
            federal_withholding_minor: a.federal_withholding_minor,
            state_withholding_minor: a.state_withholding_minor,
            net_minor: net,
            scale: a.scale,
        });
    }
    rows.sort_by(|a, b| a.occurred_on.cmp(&b.occurred_on).then(a.account_name.cmp(&b.account_name)));
    let week_gross_minor: i64 = rows.iter().map(|r| r.gross_minor).sum();
    let week_withholding_minor: i64 = rows
        .iter()
        .map(|r| r.federal_withholding_minor + r.state_withholding_minor)
        .sum();
    let week_net_minor: i64 = rows.iter().map(|r| r.net_minor).sum();
    Ok(CashManagementWeekBody {
        period_start,
        period_end,
        rows,
        week_gross_minor,
        week_withholding_minor,
        week_net_minor,
        scale: 2,
    })
}

pub async fn ssa_confirm(
    canonical: &dyn Canonical,
    account_id: uuid::Uuid,
    occurred_on: String,
    received_minor: Option<i64>,
    scale: u8,
) -> Result<ActivityRecord, PlatformError> {
    let Some(received) = received_minor else {
        return Err(PlatformError::new(
            "unknown_amount",
            "Tom Social Security retirement received amount is unknown",
        ));
    };
    let date = financial_domain::trends::parse_iso_date(&occurred_on).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid occurredOn {occurred_on}"))
    })?;
    let key = financial_domain::cash_management::tom_ssa_idempotency_key(date.year(), date.month());
    let posted = cash_distribution_post(
        canonical,
        account_id,
        "SSA".into(),
        occurred_on,
        Some(received),
        0,
        0,
        scale,
        Some(key),
    )
    .await?;
    if !financial_domain::cash_management::is_tom_ssa_amount(received) {
        let _ = canonical
            .exception_raise(
                financial_domain::cash_management::SSA_VARIANCE_CODE.into(),
                format!(
                    "Tom Social Security retirement received {} cents; expected {} cents",
                    received,
                    financial_domain::cash_management::TOM_SSA_EXPECTED_MINOR
                ),
            )
            .await;
    }
    Ok(posted)
}

pub async fn cash_management_reminders(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CashManagementRemindersBody, PlatformError> {
    let as_of_date = financial_domain::trends::parse_iso_date(as_of).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid asOfDate {as_of}"))
    })?;
    let week = financial_domain::trends::trends_period_for_capture(as_of_date);
    let period_start = week.start.format("%Y-%m-%d").to_string();
    let period_end = week.end.format("%Y-%m-%d").to_string();
    let year_month = format!("{:04}-{:02}", as_of_date.year(), as_of_date.month());
    let confirm_key = financial_domain::cash_management::tom_ssa_idempotency_key(
        as_of_date.year(),
        as_of_date.month(),
    );
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let superseded: std::collections::HashSet<_> = activities
        .iter()
        .filter_map(|a| a.corrects_activity_id)
        .collect();
    let income = accounts
        .iter()
        .find(|a| financial_domain::cash_management::is_income_account_name(&a.name));
    let external = accounts
        .iter()
        .find(|a| financial_domain::cash_management::is_ssa_account_name(&a.name));
    let income_ira_posted = activities.iter().any(|a| {
        if superseded.contains(&a.activity_id) {
            return false;
        }
        if a.activity_type != "IRA_Distribution" {
            return false;
        }
        let income_account = income
            .map(|acct| acct.account_id == a.account_id)
            .unwrap_or(false);
        income_account
            && financial_domain::income_plan::occurred_in_week(
                &a.occurred_on,
                &period_start,
                &period_end,
            )
    });
    let mut month_expected = Vec::new();
    let mut confirm_amount = None;
    let mut tom_rows: Vec<(String, i64, String, String)> = Vec::new();
    for a in &activities {
        if superseded.contains(&a.activity_id) || a.activity_type != "SSA" {
            continue;
        }
        let account_name = accounts
            .iter()
            .find(|acct| acct.account_id == a.account_id)
            .map(|acct| acct.name.clone())
            .unwrap_or_else(|| "unknown".into());
        if a.idempotency_key == confirm_key {
            confirm_amount = Some(a.amount_minor);
        }
        if financial_domain::cash_management::is_tom_ssa_amount(a.amount_minor) {
            if a.occurred_on.len() >= 7 && &a.occurred_on[..7] == year_month {
                month_expected.push(a.amount_minor);
            }
            tom_rows.push((
                a.occurred_on.clone(),
                a.amount_minor,
                account_name,
                a.occurred_on.get(..7).unwrap_or("").to_string(),
            ));
        }
    }
    tom_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut month_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let recent = tom_rows
        .into_iter()
        .map(|(occurred_on, amount_minor, account_name, ym)| {
            let count = month_counts.entry(ym).and_modify(|c| *c += 1).or_insert(1);
            CashManagementSsaRecent {
                occurred_on,
                amount_minor,
                account_name,
                extra_audit: *count >= 2,
            }
        })
        .collect();
    let (status, extra_audit, posted_minor) =
        financial_domain::cash_management::tom_ssa_month_status(
            month_expected.len(),
            confirm_amount,
        );
    Ok(CashManagementRemindersBody {
        as_of_date: as_of_date.format("%Y-%m-%d").to_string(),
        saturday_draft: CashManagementSaturdayDraft {
            open: financial_domain::cash_management::saturday_draft_open(income_ira_posted),
            activity_type: "IRA_Distribution".into(),
            suggested_account_id: income.map(|a| a.account_id),
            suggested_account_name: income
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            occurred_on: period_start,
        },
        tom_ssa: CashManagementTomSsa {
            year_month,
            expected_minor: financial_domain::cash_management::TOM_SSA_EXPECTED_MINOR,
            label: financial_domain::cash_management::SSA_RETIREMENT_LABEL.into(),
            status: status.as_str().into(),
            posted_minor,
            extra_audit,
            suggested_account_id: external.map(|a| a.account_id),
            suggested_account_name: external
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            recent,
        },
        scale: 2,
    })
}

pub async fn cash_management_month(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CashManagementMonthBody, PlatformError> {
    let as_of_date = financial_domain::trends::parse_iso_date(as_of).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid asOfDate {as_of}"))
    })?;
    let year = as_of_date.year();
    let month = as_of_date.month();
    let year_month = format!("{year:04}-{month:02}");
    let period_start = format!("{year_month}-01");
    let period_end = if month == 12 {
        format!("{year:04}-12-31")
    } else {
        let next = chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .ok_or_else(|| PlatformError::new("bad_date", "invalid month"))?;
        (next - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string()
    };
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let superseded: std::collections::HashSet<_> = activities
        .iter()
        .filter_map(|a| a.corrects_activity_id)
        .collect();
    let mut grouped: std::collections::BTreeMap<(uuid::Uuid, String), CashManagementMonthRow> =
        std::collections::BTreeMap::new();
    for a in &activities {
        if superseded.contains(&a.activity_id) {
            continue;
        }
        if !financial_domain::trends::is_non_roi_distribution(&a.activity_type) {
            continue;
        }
        if !financial_domain::cash_management::occurred_in_calendar_month(
            &a.occurred_on,
            year,
            month,
        ) {
            continue;
        }
        let account_name = accounts
            .iter()
            .find(|acct| acct.account_id == a.account_id)
            .map(|acct| acct.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let net = financial_domain::cash_management::net_minor(
            a.amount_minor,
            a.federal_withholding_minor,
            a.state_withholding_minor,
        );
        let entry = grouped
            .entry((a.account_id, a.activity_type.clone()))
            .or_insert(CashManagementMonthRow {
                account_id: a.account_id,
                account_name,
                activity_type: a.activity_type.clone(),
                count: 0,
                gross_minor: 0,
                federal_withholding_minor: 0,
                state_withholding_minor: 0,
                net_minor: 0,
                scale: a.scale,
            });
        entry.count += 1;
        entry.gross_minor += a.amount_minor;
        entry.federal_withholding_minor += a.federal_withholding_minor;
        entry.state_withholding_minor += a.state_withholding_minor;
        entry.net_minor += net;
    }
    let rows: Vec<_> = grouped.into_values().collect();
    let month_gross_minor: i64 = rows.iter().map(|r| r.gross_minor).sum();
    let month_withholding_minor: i64 = rows
        .iter()
        .map(|r| r.federal_withholding_minor + r.state_withholding_minor)
        .sum();
    let month_net_minor: i64 = rows.iter().map(|r| r.net_minor).sum();
    Ok(CashManagementMonthBody {
        year_month,
        period_start,
        period_end,
        rows,
        month_gross_minor,
        month_withholding_minor,
        month_net_minor,
        scale: 2,
    })
}
