//! Cash Management week board, Saturday draft, and Tom SSA confirm.

use crate::contracts::{
    ActivityRecord, CashManagementMonthBody, CashManagementMonthRow, CashManagementRemindersBody,
    CashManagementSaturdayDraft, CashManagementSsaPayee, CashManagementSsaRecent,
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
    let account = canonical.account_get(account_id).await?;
    financial_domain::cash_management::cash_activity_allowed_for_account(
        &activity_type,
        &account.name,
        &account.kind,
    )
    .map_err(|err| {
        let code = match err {
            financial_domain::error::DomainError::CashAccountKind => "cash_account_kind",
            financial_domain::error::DomainError::CashDistributionType => "cash_distribution_type",
            _ => "domain_error",
        };
        PlatformError::new(code, err.to_string())
    })?;
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
    let as_of = financial_domain::trends::parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
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
    rows.sort_by(|a, b| {
        a.occurred_on
            .cmp(&b.occurred_on)
            .then(a.account_name.cmp(&b.account_name))
    });
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
    payee_raw: Option<String>,
) -> Result<ActivityRecord, PlatformError> {
    let payee = payee_raw
        .as_deref()
        .and_then(financial_domain::cash_management::SsaPayee::parse)
        .unwrap_or(financial_domain::cash_management::SsaPayee::Tom);
    let Some(received) = received_minor else {
        return Err(PlatformError::new(
            "unknown_amount",
            format!(
                "{} Social Security retirement received amount is unknown",
                payee.display_name()
            ),
        ));
    };
    let date = financial_domain::trends::parse_iso_date(&occurred_on).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid occurredOn {occurred_on}"))
    })?;
    let key = financial_domain::cash_management::ssa_idempotency_key(
        payee,
        date.year(),
        date.month(),
    );
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
    if received != payee.expected_minor() {
        let _ = canonical
            .exception_raise(
                financial_domain::cash_management::SSA_VARIANCE_CODE.into(),
                format!(
                    "{} Social Security retirement received {} cents; expected {} cents",
                    payee.display_name(),
                    received,
                    payee.expected_minor()
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
    let as_of_date = financial_domain::trends::parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let week = financial_domain::trends::trends_period_for_capture(as_of_date);
    let period_start = week.start.format("%Y-%m-%d").to_string();
    let period_end = week.end.format("%Y-%m-%d").to_string();
    let year_month = format!("{:04}-{:02}", as_of_date.year(), as_of_date.month());
    let tom_confirm_key = financial_domain::cash_management::ssa_idempotency_key(
        financial_domain::cash_management::SsaPayee::Tom,
        as_of_date.year(),
        as_of_date.month(),
    );
    let barbara_confirm_key = financial_domain::cash_management::ssa_idempotency_key(
        financial_domain::cash_management::SsaPayee::Barbara,
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
    let mut month_rows: Vec<(financial_domain::cash_management::SsaPayee, i64, String)> =
        Vec::new();
    let mut unclassified_month = 0usize;
    let mut tom_confirm = None;
    let mut barbara_confirm = None;
    let mut recent_rows: Vec<(String, i64, String, String, Option<financial_domain::cash_management::SsaPayee>)> =
        Vec::new();
    for a in &activities {
        if superseded.contains(&a.activity_id) || a.activity_type != "SSA" {
            continue;
        }
        let account_name = accounts
            .iter()
            .find(|acct| acct.account_id == a.account_id)
            .map(|acct| acct.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let payee = financial_domain::cash_management::classify_ssa_row(
            &a.idempotency_key,
            a.amount_minor,
        );
        if a.idempotency_key == tom_confirm_key {
            tom_confirm = Some(a.amount_minor);
        }
        if a.idempotency_key == barbara_confirm_key {
            barbara_confirm = Some(a.amount_minor);
        }
        let in_month = a.occurred_on.len() >= 7 && &a.occurred_on[..7] == year_month;
        if in_month {
            match payee {
                Some(p) => month_rows.push((p, a.amount_minor, a.idempotency_key.clone())),
                None => unclassified_month += 1,
            }
        }
        recent_rows.push((
            a.occurred_on.clone(),
            a.amount_minor,
            account_name,
            a.occurred_on.get(..7).unwrap_or("").to_string(),
            payee,
        ));
    }
    recent_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut month_payee_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let recent = recent_rows
        .into_iter()
        .map(|(occurred_on, amount_minor, account_name, ym, payee)| {
            let bucket = format!("{}:{}", ym, payee.map(|p| p.as_str()).unwrap_or("other"));
            let count = month_payee_counts
                .entry(bucket)
                .and_modify(|c| *c += 1)
                .or_insert(1);
            CashManagementSsaRecent {
                occurred_on,
                amount_minor,
                account_name,
                extra_audit: payee.is_none() || *count >= 2,
            }
        })
        .collect();
    let tom_count = month_rows
        .iter()
        .filter(|(p, _, _)| *p == financial_domain::cash_management::SsaPayee::Tom)
        .count();
    let barbara_count = month_rows
        .iter()
        .filter(|(p, _, _)| *p == financial_domain::cash_management::SsaPayee::Barbara)
        .count();
    let (tom_status, tom_dup, tom_posted) =
        financial_domain::cash_management::ssa_payee_month_status(
            financial_domain::cash_management::SsaPayee::Tom,
            tom_count,
            tom_confirm,
        );
    let (barbara_status, barbara_dup, barbara_posted) =
        financial_domain::cash_management::ssa_payee_month_status(
            financial_domain::cash_management::SsaPayee::Barbara,
            barbara_count,
            barbara_confirm,
        );
    let extra_audit = financial_domain::cash_management::ssa_household_extra_audit(
        month_rows.len() + unclassified_month,
        tom_dup || barbara_dup,
        unclassified_month > 0,
    );
    let ssa_payees = vec![
        CashManagementSsaPayee {
            payee: "barbara".into(),
            expected_minor: financial_domain::cash_management::BARBARA_SSA_EXPECTED_MINOR,
            status: barbara_status.as_str().into(),
            posted_minor: barbara_posted,
        },
        CashManagementSsaPayee {
            payee: "tom".into(),
            expected_minor: financial_domain::cash_management::TOM_SSA_EXPECTED_MINOR,
            status: tom_status.as_str().into(),
            posted_minor: tom_posted,
        },
    ];
    Ok(CashManagementRemindersBody {
        as_of_date: as_of_date.format("%Y-%m-%d").to_string(),
        saturday_draft: CashManagementSaturdayDraft {
            open: financial_domain::cash_management::saturday_draft_open(income_ira_posted),
            activity_type: "IRA_Distribution".into(),
            suggested_account_id: income.map(|a| a.account_id),
            suggested_account_name: income.map(|a| a.name.clone()).unwrap_or_default(),
            occurred_on: period_start,
        },
        tom_ssa: CashManagementTomSsa {
            year_month,
            expected_minor: financial_domain::cash_management::TOM_SSA_EXPECTED_MINOR,
            label: financial_domain::cash_management::SSA_RETIREMENT_LABEL.into(),
            status: tom_status.as_str().into(),
            posted_minor: tom_posted,
            extra_audit,
            suggested_account_id: external.map(|a| a.account_id),
            suggested_account_name: external.map(|a| a.name.clone()).unwrap_or_default(),
            recent,
        },
        ssa_payees,
        scale: 2,
    })
}

pub async fn cash_management_month(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CashManagementMonthBody, PlatformError> {
    let as_of_date = financial_domain::trends::parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
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
