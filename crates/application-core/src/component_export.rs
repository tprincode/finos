//! One workbook per catalog module and per named part. Blank stays blank.
//! Missing money is the word unknown, never 0. Calculator money/percent are Excel types.

use rust_xlsxwriter::{Format, Workbook};

use crate::contracts::{
    AccountValueHomeBody, CashCoverageRow, CashElementListItem, CashManagementWeekRow, CashYtdRow,
    ComponentExportBody, DeclarationHistoryRowBody, DividendPlanRowBody, ExternalManagedAccount,
    ExternalRegisterLine, HoldingsLotBody, IncomePlanPositionBody, MarketImpactRowBody,
    PositionMasterRowBody, TrendsWeekPoint, WeekAheadRow,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;

struct Sheet {
    name: String,
    headers: Vec<&'static str>,
    rows: Vec<Vec<String>>,
}

/// Typed Excel cell. Calculator uses money/percent; other sheets stay Text.
#[derive(Debug, Clone)]
enum Cell {
    Text(String),
    Money { minor: i64, scale: u8 },
    Percent { bps: i64 },
    Shares { minor: i64, scale: u8 },
    Count(i64),
    Blank,
}

struct TypedSheet {
    name: String,
    headers: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

enum ExportSheet {
    Text(Sheet),
    Typed(TypedSheet),
}

pub fn export_covers(module_id: &str, part_id: &str) -> bool {
    arm(module_id, part_id).is_some()
}

pub fn export_arm(module_id: &str, part_id: &str) -> Option<&'static str> {
    arm(module_id, part_id)
}

fn arm(module_id: &str, part_id: &str) -> Option<&'static str> {
    Some(match (module_id, part_id) {
        ("graphing", "") | ("graphing", "live-by-risk") | ("trends", "live-by-risk")
        | ("trends", "live-by-risk-level") | ("trends", "live-by-risk-allocation") => "risk",
        ("graphing", "account-values") => "account-values",
        ("graphing", "graphing-period") => "graphing-period",
        ("graphing", "fidelity") => "chart-fidelity",
        ("graphing", "schwab-total") => "chart-schwab",
        ("graphing", "chart-income")
        | ("graphing", "chart-fi-roth")
        | ("graphing", "chart-car")
        | ("graphing", "chart-health")
        | ("graphing", "chart-speculation")
        | ("graphing", "chart-account-9") => "chart-book",
        ("home", "") | ("home", "dividend-plan") => "dividend-plan",
        ("home", "portfolio-summary") => "summary",
        ("home", "refresh-declarations") => "summary-declarations",
        ("home", "work-tickets") => "summary-tickets",
        ("home", "refresh-last-prices") => "summary-prices",
        ("home", "income-through") | ("home", "income-through-transactions") => "income-through",
        ("home", "account-cash-flow") => "cash-flow",
        ("trends", "declared-vs-plan") | ("income-plan", "plan-vs-decl") => "declared-vs-plan",
        ("trends", "planned-weekly-income") => "planned-week",
        ("trends", "reported-weekly-income") => "reported-week",
        ("trends", "dividends-paid-by-month") | ("trends", "monthly-dividends") => "monthly-divs",
        ("trends", "all-cash") => "all-cash",
        ("trends", "fidelity-schwab") => "fid-sch",
        ("week-ahead", "week-ahead-tasks")
        | ("week-ahead", "week-ahead-elements")
        | ("week-ahead", "week-ahead-loans") => "week-ahead",
        ("cm-coverage", "coverage-income")
        | ("cm-coverage", "coverage-expense")
        | ("cm-coverage", "coverage-plan") => "coverage",
        ("import", "import-step") | ("import", "validate-step") | ("import", "loaded-step") => "import",
        ("calculator", "") | ("calculator", "calculator-sheet") => "calculator",
        ("trends", "") | ("trends", "trends-weeks") => "trends",
        ("income-plan", "") => "income-plan",
        ("market-impact", "") | ("market-impact", "market-impact-windows") => "market-impact",
        ("dashboard", "") => "dashboard",
        ("position-details", "") => "position-master",
        ("holdings", "")
        | ("holdings", "holdings-panel")
        | ("holdings", "open-lots")
        | ("lots", "")
        | ("add-lot", "") => "holdings",
        ("shopping-cart", "") => "cart",
        ("cash-management", "") | ("cash-week-desk", "") | ("cm-cashflow-manager", "") => "cash-week",
        ("week-ahead", "") | ("cm-weekly-updates", "") => "week-ahead",
        ("household-income", "") | ("magi-forecast", "") | ("cm-car-account-tax", "") | ("cash-management-ytd", "") => {
            "cash-ytd"
        }
        ("cash-elements", "") | ("cm-element-management", "") => "elements",
        ("cm-coverage", "") => "coverage",
        ("cm-external", "") | ("cm-external", "cct-register") => "register",
        ("cm-debt-planner", "")
        | ("cm-debt-planner", "debt-accounts")
        | ("cm-debt-planner", "debt-buckets") => "debt",
        ("account-management", "") | ("account-management", "brokerage-accounts") => "accounts",
        ("new-investment-readiness", "") | ("add-position", "") | ("collectors", "") | ("reevaluate-collector", "") => {
            "securities"
        }
        ("contract-positions", "")
        | ("contract-positions", "contract-create")
        | ("contract-positions", "contract-open")
        | ("contract-positions", "contract-closed") => "contracts",
        ("task-manager", "") | ("tickets", "") => "tasks",
        ("import", "") => "import",
        ("interest-rate", "") | ("interest-rate", "period-conversion") => "interest",
        ("field-intent", "") | ("field-intent", "calculator-columns") => "field-intent",
        ("roadmap", "") | ("roadmap", "cash-covered-puts") => "catalog",
        ("settings", "") | ("components", "") | ("screen-atlas", "") | ("shell", "") => "catalog",
        _ => return None,
    })
}

pub async fn export_component(
    canonical: &dyn Canonical,
    module_id: &str,
    part_id: &str,
    as_of: &str,
) -> Result<ComponentExportBody, PlatformError> {
    let kind = arm(module_id, part_id).ok_or_else(|| {
        PlatformError::new(
            "export_not_proven",
            format!("no export arm for {module_id} {part_id}"),
        )
    })?;
    let title = sheet_title(module_id, part_id);
    let sheet = if kind == "calculator" {
        let mut typed = calculator_typed_sheet(canonical, as_of).await?;
        typed.name = title;
        ExportSheet::Typed(typed)
    } else {
        let built = build_kind_sheet(canonical, module_id, part_id, as_of, kind).await?;
        ExportSheet::Text(Sheet {
            name: title,
            headers: built.headers,
            rows: built.rows,
        })
    };
    let bytes = export_workbook_bytes(std::slice::from_ref(&sheet))
        .map_err(|e| PlatformError::new("export_failed", e))?;
    let stem = if part_id.is_empty() {
        module_id.to_string()
    } else {
        format!("{module_id}-{part_id}")
    };
    Ok(ComponentExportBody {
        default_file_name: format!("{stem}.xlsx"),
        bytes_base64: crate::cash_register::b64_encode(&bytes),
    })
}

async fn build_kind_sheet(
    canonical: &dyn Canonical,
    module_id: &str,
    part_id: &str,
    as_of: &str,
    kind: &str,
) -> Result<Sheet, PlatformError> {
    match kind {
        "risk" => risk_sheet(canonical, as_of).await,
        "account-values" => account_values_sheet(canonical, as_of).await,
        "dividend-plan" => dividend_sheet(canonical, as_of).await,
        "summary" => portfolio_sheet(canonical, as_of).await,
        "summary-declarations" => summary_slice(canonical, as_of, "declarations").await,
        "summary-tickets" => summary_slice(canonical, as_of, "tickets").await,
        "summary-prices" => summary_slice(canonical, as_of, "prices").await,
        "income-through" => income_through_sheet(canonical).await,
        "cash-flow" => cash_flow_sheet(canonical, as_of).await,
        "graphing-period" => graphing_period_sheet(canonical, as_of).await,
        "chart-fidelity" => one_series_sheet(canonical, as_of, "fidelity").await,
        "chart-schwab" => one_series_sheet(canonical, as_of, "schwab").await,
        "chart-book" => one_series_sheet(canonical, as_of, part_id).await,
        "declared-vs-plan" => performance_sheet(canonical, as_of, "both").await,
        "planned-week" => performance_sheet(canonical, as_of, "plan").await,
        "reported-week" => performance_sheet(canonical, as_of, "actual").await,
        "monthly-divs" => week_metric_sheet(canonical, as_of, "monthly").await,
        "all-cash" => week_metric_sheet(canonical, as_of, "cash").await,
        "fid-sch" => week_metric_sheet(canonical, as_of, "fidsch").await,
        "calculator" => {
            // Typed path is taken in export_component / export_page. This arm
            // stays for the string Sheet contract used by unique_sheet_names tests.
            calculator_legacy_sheet(canonical).await
        }
        "trends" => trends_sheet(canonical, as_of).await,
        "income-plan" => income_plan_sheet(canonical, as_of).await,
        "market-impact" => market_impact_sheet(canonical).await,
        "dashboard" => dashboard_sheet(canonical, as_of).await,
        "position-master" => position_master_sheet(canonical).await,
        "holdings" => holdings_sheet(canonical).await,
        "cart" => cart_sheet(canonical).await,
        "cash-week" => cash_week_sheet(canonical, as_of).await,
        "week-ahead" => week_ahead_sheet(canonical, as_of).await,
        "cash-ytd" => cash_ytd_sheet(canonical, as_of).await,
        "elements" => elements_sheet(canonical, as_of).await,
        "coverage" => coverage_sheet(canonical, as_of).await,
        "register" => register_sheet(canonical).await,
        "debt" => debt_sheet(canonical).await,
        "accounts" => accounts_sheet(canonical).await,
        "securities" => securities_sheet(canonical).await,
        "contracts" => contracts_sheet(canonical).await,
        "tasks" => tasks_sheet(canonical).await,
        "import" => import_sheet(canonical).await,
        "interest" => Ok(interest_sheet()),
        "field-intent" => Ok(field_intent_sheet()),
        "catalog" => Ok(catalog_sheet(module_id)),
        _ => Err(PlatformError::new(
            "export_not_proven",
            format!("unmapped kind {kind}"),
        )),
    }
}

fn sheet_title(module_id: &str, part_id: &str) -> String {
    let Ok(catalog) = crate::core_functions::core_functions_catalog() else {
        return module_id.to_string();
    };
    let Some(module) = catalog.modules.iter().find(|m| m.id == module_id) else {
        return module_id.to_string();
    };
    if part_id.is_empty() {
        return module.title.clone();
    }
    module
        .parts
        .iter()
        .find(|p| p.id == part_id)
        .map(|p| p.title.clone())
        .unwrap_or_else(|| module.title.clone())
}

fn money_text(minor: i64, scale: u8) -> String {
    let scale = u32::from(scale.min(8));
    let div = 10i64.pow(scale);
    let sign = if minor < 0 { "-" } else { "" };
    let v = minor.abs();
    if scale == 0 {
        return format!("{sign}{v}");
    }
    let whole = v / div;
    let frac = v % div;
    format!("{sign}{whole}.{frac:0width$}", width = scale as usize)
}

fn money_unknown(minor: Option<i64>, scale: u8) -> String {
    match minor {
        None => "unknown".into(),
        Some(v) => money_text(v, scale),
    }
}

fn known_money(known: bool, minor: i64, scale: u8) -> String {
    if known {
        money_text(minor, scale)
    } else {
        "unknown".into()
    }
}

fn bps_text(bps: Option<i64>) -> String {
    match bps {
        None => "unknown".into(),
        Some(v) => {
            let sign = if v < 0 { "-" } else { "" };
            let a = v.abs();
            format!("{sign}{}.{:02}%", a / 100, a % 100)
        }
    }
}

fn sheet_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '\\' | '/' | '*' | '?' | ':' | '[' | ']' => ' ',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim();
    let name = if trimmed.is_empty() { "Sheet" } else { trimmed };
    name.chars().take(31).collect()
}

#[cfg(test)]
fn workbook_bytes(sheets: &[Sheet]) -> Result<Vec<u8>, String> {
    let export: Vec<ExportSheet> = sheets
        .iter()
        .map(|s| {
            ExportSheet::Text(Sheet {
                name: s.name.clone(),
                headers: s.headers.clone(),
                rows: s.rows.clone(),
            })
        })
        .collect();
    export_workbook_bytes(&export)
}

fn export_workbook_bytes(sheets: &[ExportSheet]) -> Result<Vec<u8>, String> {
    let mut wb = Workbook::new();
    let names = unique_export_sheet_names(sheets);
    let money2 = Format::new().set_num_format("$#,##0.00");
    let percent = Format::new().set_num_format("0.00%");
    for (sheet, name) in sheets.iter().zip(names) {
        let ws = wb.add_worksheet();
        ws.set_name(&name).map_err(|e| e.to_string())?;
        match sheet {
            ExportSheet::Text(sheet) => {
                for (c, h) in sheet.headers.iter().enumerate() {
                    ws.write_string(0, c as u16, *h)
                        .map_err(|e| e.to_string())?;
                }
                for (r, row) in sheet.rows.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        ws.write_string((r + 1) as u32, c as u16, cell)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            ExportSheet::Typed(sheet) => {
                for (c, h) in sheet.headers.iter().enumerate() {
                    ws.write_string(0, c as u16, h)
                        .map_err(|e| e.to_string())?;
                }
                for (r, row) in sheet.rows.iter().enumerate() {
                    let excel_row = (r + 1) as u32;
                    for (c, cell) in row.iter().enumerate() {
                        let col = c as u16;
                        match cell {
                            Cell::Text(text) => {
                                ws.write_string(excel_row, col, text)
                                    .map_err(|e| e.to_string())?;
                            }
                            Cell::Blank => {}
                            Cell::Money { minor, scale } => {
                                let major = *minor as f64 / 10f64.powi(i32::from(*scale));
                                let fmt = if *scale == 2 {
                                    money2.clone()
                                } else if *scale == 0 {
                                    Format::new().set_num_format("$#,##0")
                                } else {
                                    Format::new().set_num_format(format!(
                                        "$#,##0.{}",
                                        "0".repeat(*scale as usize)
                                    ))
                                };
                                ws.write_number_with_format(excel_row, col, major, &fmt)
                                    .map_err(|e| e.to_string())?;
                            }
                            Cell::Percent { bps } => {
                                let value = *bps as f64 / 10_000.0;
                                ws.write_number_with_format(excel_row, col, value, &percent)
                                    .map_err(|e| e.to_string())?;
                            }
                            Cell::Shares { minor, scale } => {
                                let value = *minor as f64 / 10f64.powi(i32::from(*scale));
                                ws.write_number(excel_row, col, value)
                                    .map_err(|e| e.to_string())?;
                            }
                            Cell::Count(n) => {
                                ws.write_number(excel_row, col, *n as f64)
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                }
            }
        }
    }
    wb.save_to_buffer().map_err(|e| e.to_string())
}

#[cfg(test)]
fn unique_sheet_names(sheets: &[Sheet]) -> Vec<String> {
    let export: Vec<ExportSheet> = sheets
        .iter()
        .map(|s| {
            ExportSheet::Text(Sheet {
                name: s.name.clone(),
                headers: s.headers.clone(),
                rows: s.rows.clone(),
            })
        })
        .collect();
    unique_export_sheet_names(&export)
}

fn unique_export_sheet_names(sheets: &[ExportSheet]) -> Vec<String> {
    let mut used = std::collections::HashSet::new();
    let mut names = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let title = match sheet {
            ExportSheet::Text(s) => s.name.as_str(),
            ExportSheet::Typed(s) => s.name.as_str(),
        };
        let base = sheet_name(title);
        let mut name = base.clone();
        let mut n = 2u32;
        while !used.insert(name.clone()) {
            let suffix = format!(" {n}");
            let keep = 31usize.saturating_sub(suffix.chars().count());
            let stem: String = base.chars().take(keep).collect();
            name = format!("{stem}{suffix}");
            n += 1;
        }
        names.push(name);
    }
    names
}

fn page_file_name(label: &str) -> String {
    let cleaned: String = label
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim();
    let stem = if trimmed.is_empty() { "page" } else { trimmed };
    format!("{stem}.xlsx")
}

pub async fn export_page(
    canonical: &dyn Canonical,
    page_label: &str,
    lines: &[(String, String)],
    as_of: &str,
) -> Result<ComponentExportBody, PlatformError> {
    if lines.is_empty() {
        return Err(PlatformError::new(
            "export_not_proven",
            "page has no components",
        ));
    }
    let mut sheets = Vec::with_capacity(lines.len());
    for (module_id, part_id) in lines {
        let kind = arm(module_id, part_id).ok_or_else(|| {
            PlatformError::new(
                "export_not_proven",
                format!("no export arm for {module_id} {part_id}"),
            )
        })?;
        let title = sheet_title(module_id, part_id);
        if kind == "calculator" {
            let mut typed = calculator_typed_sheet(canonical, as_of).await?;
            typed.name = title;
            sheets.push(ExportSheet::Typed(typed));
        } else {
            let built = build_kind_sheet(canonical, module_id, part_id, as_of, kind).await?;
            sheets.push(ExportSheet::Text(Sheet {
                name: title,
                headers: built.headers,
                rows: built.rows,
            }));
        }
    }
    let bytes =
        export_workbook_bytes(&sheets).map_err(|e| PlatformError::new("export_failed", e))?;
    Ok(ComponentExportBody {
        default_file_name: page_file_name(page_label),
        bytes_base64: crate::cash_register::b64_encode(&bytes),
    })
}

async fn home_values(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<AccountValueHomeBody, PlatformError> {
    crate::account_value::account_value_home_view(canonical, as_of).await
}

async fn risk_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let rows = home
        .risk
        .groups
        .iter()
        .map(|group| {
            vec![
                group.risk_tier.clone(),
                money_unknown(group.current_minor, home.scale),
                home.as_of.clone(),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Live by risk".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

async fn account_values_sheet(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let mut rows = Vec::new();
    for series in &home.accounts {
        for point in &series.points {
            rows.push(vec![
                series.account_name.clone(),
                money_unknown(point.market_value_minor, series.scale),
                point.as_of.clone(),
            ]);
        }
    }
    Ok(Sheet {
        name: "Account values".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

fn dividend_rows(rows: &[DividendPlanRowBody], scale: u8) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                row.account_name.clone(),
                money_unknown(row.annual_dividend_minor, scale),
                money_unknown(row.market_value_minor, scale),
                money_unknown(row.monthly_income_minor, scale),
                money_unknown(row.monthly_medical_minor, scale),
                money_unknown(row.weekly_minor, scale),
                bps_text(row.effective_annual_bps),
            ]
        })
        .collect()
}

async fn dividend_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let plan = crate::dividend_plan::dividend_plan_home_view(canonical, as_of).await?;
    let mut rows = dividend_rows(&plan.rows, plan.scale);
    rows.push(dividend_rows(std::slice::from_ref(&plan.total), plan.scale).pop().unwrap_or_default());
    Ok(Sheet {
        name: "Dividend Plan".into(),
        headers: vec![
            "Account",
            "Annual dividend",
            "Market value",
            "Monthly Income",
            "Monthly Medical",
            "Weekly",
            "Effective annual return",
        ],
        rows,
    })
}

const CASH_GRID: &[(&str, &str)] = &[
    ("Income", "SPAXX"),
    ("FI Roth", "SPAXX"),
    ("Speculation", "SPAXX"),
    ("Car", "SPAXX"),
    ("Health", "FDRXX"),
    ("9", "SWVXX"),
];

async fn portfolio_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let summary = crate::queries::data_summary_view_with_avgs(canonical, as_of, None).await?;
    let scale = summary.scale;
    let unrealized = match summary.market_value_minor {
        None => "unknown".to_string(),
        Some(mv) => money_text(mv - summary.open_performance_minor, scale),
    };
    let mut rows = vec![
        vec!["Market value".into(), money_unknown(summary.market_value_minor, scale)],
        vec!["Original cost".into(), money_text(summary.open_performance_minor, scale)],
        vec!["Tax basis".into(), money_text(summary.open_tax_minor, scale)],
        vec!["Unrealized".into(), unrealized],
        vec!["Income earned".into(), money_text(summary.income_earned_minor, scale)],
        vec![
            "Average previous 12 months".into(),
            money_unknown(summary.avg_monthly_actual_income_minor, scale),
        ],
        vec![
            "Average Monthly Plan".into(),
            money_unknown(summary.avg_monthly_plan_income_minor, scale),
        ],
        vec![
            "Dividend Managed positions".into(),
            format!("{} of {}", summary.declaration_count, summary.declaration_collector_count),
        ],
        vec!["Open tickets".into(), summary.open_ticket_count.to_string()],
        vec![
            "Last Price".into(),
            format!("{} of {}", summary.last_price_count, summary.symbol_count),
        ],
    ];
    let holdings = crate::queries::holdings_view(canonical).await?;
    for (account, symbol) in CASH_GRID {
        let label = if *account == "9" { "Account 9 cash" } else { account };
        let found = holdings.lots.iter().find(|lot| {
            let account_name = if *account == "9" {
                lot.account_name == "9" || lot.account_name == "Account 9"
            } else {
                lot.account_name == *account
            };
            account_name
                && lot.symbol.eq_ignore_ascii_case(symbol)
                && lot.remaining_quantity_minor > 0
        });
        let value = match found {
            Some(lot) => money_text(lot.remaining_quantity_minor, lot.quantity_scale),
            None => "none".into(),
        };
        rows.push(vec![format!("{label} cash"), value]);
    }
    Ok(Sheet {
        name: "Portfolio summary".into(),
        headers: vec!["Label", "Value"],
        rows,
    })
}

async fn summary_slice(
    canonical: &dyn Canonical,
    as_of: &str,
    which: &str,
) -> Result<Sheet, PlatformError> {
    let summary = crate::queries::data_summary_view_with_avgs(canonical, as_of, None).await?;
    let rows = match which {
        "declarations" => vec![
            vec!["Dividend Managed positions".into(), summary.declaration_count.to_string()],
            vec!["Collectors".into(), summary.declaration_collector_count.to_string()],
            vec!["Last update".into(), present_or_none(summary.declaration_refreshed_on.clone())],
        ],
        "tickets" => vec![vec!["Open tickets".into(), summary.open_ticket_count.to_string()]],
        _ => vec![
            vec!["Last Price".into(), summary.last_price_count.to_string()],
            vec!["Symbols".into(), summary.symbol_count.to_string()],
            vec!["Last refresh".into(), present_or_none(summary.last_price_refreshed_on.clone())],
        ],
    };
    Ok(Sheet {
        name: "Portfolio summary".into(),
        headers: vec!["Label", "Value"],
        rows,
    })
}

async fn income_through_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let dividend = canonical.dividend_get().await?;
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let mut rows = Vec::new();
    for actual in dividend.actuals {
        let account = accounts
            .iter()
            .find(|account| account.account_id == actual.account_id)
            .map(|account| account.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let symbol = match actual.security_id {
            Some(id) => securities
                .iter()
                .find(|security| security.security_id == id)
                .map(|security| security.symbol.clone())
                .unwrap_or_else(|| "unknown".into()),
            None => "unknown".into(),
        };
        rows.push(vec![
            actual.occurred_on,
            account,
            symbol,
            money_text(actual.amount_minor, actual.scale),
        ]);
    }
    Ok(Sheet {
        name: "Income through transactions".into(),
        headers: vec!["Date", "Acct", "Symbol", "Amount"],
        rows,
    })
}

fn present_or_none(value: Option<String>) -> String {
    value
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| "none".into())
}

fn series_rows(name: &str, points: &[crate::contracts::AccountValuePointBody], scale: u8) -> Vec<Vec<String>> {
    points
        .iter()
        .map(|point| {
            vec![
                name.to_string(),
                money_unknown(point.market_value_minor, scale),
                point.as_of.clone(),
            ]
        })
        .collect()
}

async fn one_series_sheet(
    canonical: &dyn Canonical,
    as_of: &str,
    which: &str,
) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let book = match which {
        "chart-income" => "Income",
        "chart-fi-roth" => "FI Roth",
        "chart-car" => "Car",
        "chart-health" => "Health",
        "chart-speculation" => "Speculation",
        "chart-account-9" => "Account 9",
        _ => "",
    };
    let rows = if which == "fidelity" {
        series_rows(&home.fidelity.account_name, &home.fidelity.points, home.fidelity.scale)
    } else if which == "schwab" {
        series_rows(&home.schwab.account_name, &home.schwab.points, home.schwab.scale)
    } else {
        match home.accounts.iter().find(|series| series.account_name == book) {
            Some(series) => series_rows(&series.account_name, &series.points, series.scale),
            None => Vec::new(),
        }
    };
    Ok(Sheet {
        name: "Account values".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

async fn graphing_period_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    Ok(Sheet {
        name: "Graphing period".into(),
        headers: vec!["Label", "Value"],
        rows: vec![
            vec!["As of".into(), home.as_of],
            vec!["Fidelity points".into(), home.fidelity.points.len().to_string()],
            vec!["Schwab points".into(), home.schwab.points.len().to_string()],
            vec!["Accounts".into(), home.accounts.len().to_string()],
        ],
    })
}

async fn cash_flow_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let mut rows = Vec::new();
    for week in &home.weeks {
        let scale = week.scale;
        let end = week.period_end.clone();
        rows.push(vec!["Income".into(), money_text(week.income_cash_minor, scale), end.clone()]);
        rows.push(vec!["FI Roth".into(), money_unknown(week.roth_cash_minor, scale), end.clone()]);
        rows.push(vec!["Car".into(), money_unknown(week.car_cash_minor, scale), end.clone()]);
        rows.push(vec!["Health".into(), money_unknown(week.health_cash_minor, scale), end.clone()]);
        rows.push(vec![
            "Speculation".into(),
            money_unknown(week.speculation_cash_minor, scale),
            end.clone(),
        ]);
        rows.push(vec!["Account 9".into(), money_text(week.acct9_cash_minor, scale), end]);
    }
    Ok(Sheet {
        name: "Account cash flow projection".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

async fn performance_sheet(
    canonical: &dyn Canonical,
    as_of: &str,
    which: &str,
) -> Result<Sheet, PlatformError> {
    let body = crate::queries::dividend_performance_view(canonical, as_of.to_string(), "ytd".into()).await?;
    let rows = body
        .weeks
        .iter()
        .map(|week| {
            let plan = if week.plan_known {
                money_text(week.planned_minor, week.scale)
            } else {
                "unknown".into()
            };
            let declared = if week.declaration_known {
                money_text(week.declaration_minor, week.scale)
            } else {
                "unknown".into()
            };
            let actual = if week.actual_known {
                money_text(week.actual_minor, week.scale)
            } else {
                "unknown".into()
            };
            match which {
                "plan" => vec![week.end.clone(), plan],
                "actual" => vec![week.end.clone(), actual],
                _ => vec![week.end.clone(), plan, declared, actual],
            }
        })
        .collect();
    let headers = match which {
        "plan" => vec!["Week", "Plan"],
        "actual" => vec!["Week", "Actual"],
        _ => vec!["Week", "Plan", "Declared", "Actual"],
    };
    Ok(Sheet {
        name: "Declared vs Plan".into(),
        headers,
        rows,
    })
}

async fn week_metric_sheet(
    canonical: &dyn Canonical,
    as_of: &str,
    which: &str,
) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let rows = home
        .weeks
        .iter()
        .map(|week| {
            let value = match which {
                "cash" => money_text(week.total_cash_minor, week.scale),
                "fidsch" => money_text(week.fid_sch_combined_minor, week.scale),
                _ => money_text(week.monthly_divs_minor, week.scale),
            };
            vec![week.period_end.clone(), value, week.period_end.clone()]
        })
        .collect();
    Ok(Sheet {
        name: "Trends".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

async fn calculator_legacy_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::queries::calculator_view(canonical).await?;
    let rows = body
        .rows
        .iter()
        .map(|row| {
            vec![
                row.symbol.clone(),
                money_unknown(row.last_price_minor, row.last_price_scale.unwrap_or(2)),
                row.payment_frequency.clone(),
                money_text(row.remaining_quantity_minor, row.quantity_scale),
                if row.plan_known {
                    money_text(row.plan_payment_minor, row.plan_scale)
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Calculator".into(),
        headers: vec!["Symbol", "Price", "Type", "Shares", "Plan"],
        rows,
    })
}

fn money_cell(minor: Option<i64>, scale: u8) -> Cell {
    match minor {
        None => Cell::Blank,
        Some(v) => Cell::Money { minor: v, scale },
    }
}

fn percent_cell(bps: Option<i64>) -> Cell {
    match bps {
        None => Cell::Blank,
        Some(v) => Cell::Percent { bps: v },
    }
}

fn text_or_dash(value: &str) -> Cell {
    let t = value.trim();
    if t.is_empty() {
        Cell::Text("—".into())
    } else {
        Cell::Text(t.to_string())
    }
}

/// Full live Calculator sheet: field-intent columns + dated week heads. DIV-1 only.
async fn calculator_typed_sheet(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<TypedSheet, PlatformError> {
    let master = crate::queries::position_master_view(canonical).await?;
    let history = crate::queries::declaration_history_view(
        canonical,
        as_of,
        "all",
        None,
        Some(as_of),
        None,
    )
    .await?;
    let history_by: std::collections::HashMap<&str, &DeclarationHistoryRowBody> = history
        .rows
        .iter()
        .map(|row| (row.symbol.as_str(), row))
        .collect();
    let household_ex_cash: i64 = master
        .rows
        .iter()
        .filter(|row| !row.cash_par)
        .filter_map(|row| row.market_value_minor)
        .sum();
    let mut headers: Vec<String> = calculator_static_headers()
        .iter()
        .map(|h| (*h).to_string())
        .collect();
    for friday in &history.week_ends {
        headers.push(friday.clone());
    }
    let mut rows = Vec::new();
    for row in &master.rows {
        if row.cash_par {
            continue;
        }
        if !financial_domain::collector::calculator_view_includes(
            &row.div_type,
            &row.symbol,
            &row.payment_frequency,
        ) {
            continue;
        }
        if row.remaining_quantity_minor <= 0 && !row.plan_known {
            continue;
        }
        let hist = history_by.get(row.symbol.as_str()).copied();
        rows.push(calculator_typed_row(
            row,
            hist,
            household_ex_cash,
            history.week_ends.len(),
        ));
    }
    Ok(TypedSheet {
        name: "Calculator".into(),
        headers,
        rows,
    })
}

fn calculator_static_headers() -> &'static [&'static str] {
    &[
        "Symbol",
        "Price",
        "Port %",
        "Risk",
        "Account",
        "Shares",
        "Cost",
        "Avg px",
        "Gain $",
        "Gain %",
        "YOC",
        "FWD",
        "MC FWD",
        "Annual",
        "Plan",
        "Type",
        "Sched",
        "Div recv",
        "ROC $",
        "Cost rec",
        "ROC %",
        "Declares",
        "Ex-date",
        "Payday",
        "Decl freshness",
        "Decl count",
        "Ann / share",
        "Recent / share",
        "Recent total",
        "Realized %",
        "Calculator blend",
        "MC TVAL",
        "TVAL Δ",
        "3-pay yield",
        "Plan Δ",
        "Over/Under",
        "Plan pay",
        "Current pay",
        "Most current",
        "Avg 3",
        "Avg 6",
        "Paid",
        "Plan check",
    ]
}

fn calculator_typed_row(
    row: &PositionMasterRowBody,
    hist: Option<&DeclarationHistoryRowBody>,
    household_ex_cash: i64,
    week_count: usize,
) -> Vec<Cell> {
    let scale = row.scale;
    let price_scale = row.last_price_scale.unwrap_or(2);
    let port_bps = if household_ex_cash > 0 {
        row.market_value_minor
            .map(|mv| ((mv as i128 * 10_000) / household_ex_cash as i128) as i64)
    } else {
        None
    };
    let gain = row
        .market_value_minor
        .map(|mv| mv - row.remaining_performance_minor);
    let blend = match (row.unrealized_pnl_bps, row.plan_fwd_yield_bps) {
        (Some(pnl), Some(fwd)) => Some((pnl * 2 + fwd) / 2),
        _ => None,
    };
    let mc_blend = match (row.unrealized_pnl_bps, row.most_current_fwd_yield_bps) {
        (Some(pnl), Some(fwd)) => Some((pnl * 2 + fwd) / 2),
        _ => None,
    };
    let tval_delta = match (mc_blend, blend) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    };
    let newest = hist.and_then(|h| h.recent_pays.first());
    let newest_minor = newest.and_then(|p| p.amount_per_share_minor);
    let newest_scale = newest.map(|p| p.amount_scale).unwrap_or(4);
    let plan_pay = if row.plan_known && row.remaining_quantity_minor > 0 {
        Some(lot_times_per_share(
            row.plan_per_share_minor,
            row.plan_scale,
            row.remaining_quantity_minor,
            row.quantity_scale,
        ))
    } else {
        None
    };
    let current_pay = newest_minor.map(|per| {
        lot_times_per_share(
            per,
            newest_scale,
            row.remaining_quantity_minor,
            row.quantity_scale,
        )
    });
    let plan_delta = match (newest_minor, row.plan_known) {
        (Some(per), true) => {
            let decl = lot_times_per_share(
                per,
                newest_scale,
                row.remaining_quantity_minor,
                row.quantity_scale,
            );
            let plan = plan_pay.unwrap_or(0);
            Some(decl - plan)
        }
        _ => None,
    };
    let over_under = match (newest_minor, row.plan_known) {
        (Some(per), true) if row.plan_per_share_minor != 0 => {
            let left = scale_to_scale4(per, newest_scale);
            let right = scale_to_scale4(row.plan_per_share_minor, row.plan_scale);
            if right == 0 {
                None
            } else {
                Some(((left - right) * 10_000) / right)
            }
        }
        _ => None,
    };
    let ann_share = if row.plan_known {
        Some(row.plan_per_share_minor.saturating_mul(i64::from(
            periods_per_year(&row.payment_frequency).unwrap_or(0),
        )))
    } else {
        None
    };
    let recent_share = newest_minor.map(|per| {
        per.saturating_mul(i64::from(
            periods_per_year(&row.payment_frequency).unwrap_or(0),
        ))
    });
    let recent_total = recent_share.map(|per_year| {
        lot_times_per_share(
            per_year,
            newest_scale,
            row.remaining_quantity_minor,
            row.quantity_scale,
        )
    });
    let three_yield = three_pay_yield_bps(hist, row.last_price_minor, &row.payment_frequency);
    let paid = hist.and_then(|h| {
        let mut sum: i64 = 0;
        let mut any = false;
        for cell in &h.cells {
            if let Some(amt) = cell.amount_per_share_minor {
                sum += scale_to_cents(amt, cell.amount_scale);
                any = true;
            }
        }
        if any {
            Some(sum)
        } else {
            None
        }
    });
    let plan_check = plan_check_text(hist, row.plan_known, row.plan_per_share_minor, row.plan_scale);
    let avg3 = hist.and_then(|h| h.avg3_minor.map(|m| (m, h.avg3_scale)));
    let avg6 = hist.and_then(|h| h.avg6_minor.map(|m| (m, h.avg6_scale)));
    let roc = calculator_roc_percent(row);

    let mut cells = vec![
        Cell::Text(row.symbol.clone()),
        money_cell(row.last_price_minor, price_scale),
        percent_cell(port_bps),
        text_or_dash(&row.risk_tier),
        Cell::Text("—".into()),
        Cell::Shares {
            minor: row.remaining_quantity_minor,
            scale: row.quantity_scale,
        },
        Cell::Money {
            minor: row.remaining_performance_minor,
            scale,
        },
        money_cell(row.unit_cost_minor, 2),
        money_cell(gain, scale),
        percent_cell(row.unrealized_pnl_bps),
        percent_cell(row.plan_yoc_bps),
        percent_cell(row.plan_fwd_yield_bps),
        percent_cell(row.most_current_fwd_yield_bps),
        money_cell(row.annual_plan_minor, scale),
        if row.plan_known {
            Cell::Money {
                minor: row.plan_per_share_minor,
                scale: row.plan_scale,
            }
        } else {
            Cell::Blank
        },
        text_or_dash(&row.div_type),
        text_or_dash(&row.payment_frequency),
        if row.distributions_scope == "incomplete" {
            Cell::Blank
        } else {
            money_cell(row.total_distributions_received_minor, scale)
        },
        if row.distributions_scope == "incomplete" {
            Cell::Blank
        } else {
            money_cell(row.roc_distributions_minor, scale)
        },
        percent_cell(row.cost_recovery_bps),
        match roc {
            Some((minor, roc_scale)) => {
                // Display percent: minor/10^scale is already a percent number (e.g. 99.7).
                // Excel 0.00% wants a fraction, so convert percent → bps → fraction.
                let bps = ((minor as f64 / 10f64.powi(i32::from(roc_scale))) * 100.0).round() as i64;
                Cell::Percent { bps }
            }
            None => Cell::Blank,
        },
        text_or_dash(&row.declaration_weekday),
        text_or_dash(&row.exdate_weekday),
        text_or_dash(&row.payday_weekday),
        text_or_dash(&row.declaration_freshness),
        Cell::Count(row.declaration_count as i64),
        money_cell(ann_share, row.plan_scale),
        money_cell(recent_share, newest_scale),
        money_cell(recent_total, 2),
        percent_cell(row.cost_recovery_bps),
        percent_cell(blend),
        percent_cell(mc_blend),
        percent_cell(tval_delta),
        percent_cell(three_yield),
        money_cell(plan_delta, 2),
        percent_cell(over_under),
        money_cell(plan_pay, 2),
        money_cell(current_pay, 2),
        money_cell(newest_minor, newest_scale),
        match avg3 {
            Some((minor, scale)) => Cell::Money { minor, scale },
            None => Cell::Blank,
        },
        match avg6 {
            Some((minor, scale)) => Cell::Money { minor, scale },
            None => Cell::Blank,
        },
        money_cell(paid, 2),
        Cell::Text(plan_check),
    ];
    let empty_weeks = week_count;
    if let Some(h) = hist {
        for cell in &h.cells {
            cells.push(match cell.amount_per_share_minor {
                None => Cell::Blank,
                Some(amt) => Cell::Money {
                    minor: amt,
                    scale: cell.amount_scale,
                },
            });
        }
        for _ in h.cells.len()..empty_weeks {
            cells.push(Cell::Blank);
        }
    } else {
        for _ in 0..empty_weeks {
            cells.push(Cell::Blank);
        }
    }
    cells
}

fn periods_per_year(freq: &str) -> Option<u8> {
    match freq.trim().to_ascii_lowercase().as_str() {
        "weekly" => Some(52),
        "monthly" => Some(12),
        "quarterly" => Some(4),
        "twice monthly" | "twice-monthly" | "semimonthly" => Some(24),
        "annual" | "yearly" => Some(1),
        _ => None,
    }
}

fn scale_to_cents(minor: i64, scale: u8) -> i64 {
    if scale <= 2 {
        minor * 10i64.pow(u32::from(2 - scale))
    } else {
        minor / 10i64.pow(u32::from(scale - 2))
    }
}

fn scale_to_scale4(minor: i64, scale: u8) -> i64 {
    if scale <= 4 {
        minor * 10i64.pow(u32::from(4 - scale))
    } else {
        minor / 10i64.pow(u32::from(scale - 4))
    }
}

fn lot_times_per_share(
    per_share_minor: i64,
    per_share_scale: u8,
    qty_minor: i64,
    qty_scale: u8,
) -> i64 {
    // Result in cents (scale 2): (per * qty) / 10^(per_scale + qty_scale - 2)
    let num = (per_share_minor as i128) * (qty_minor as i128);
    let denom_exp = i32::from(per_share_scale) + i32::from(qty_scale) - 2;
    if denom_exp >= 0 {
        (num / 10i128.pow(denom_exp as u32)) as i64
    } else {
        (num * 10i128.pow((-denom_exp) as u32)) as i64
    }
}

fn calculator_roc_percent(row: &PositionMasterRowBody) -> Option<(i64, u8)> {
    let scale = row.roc_scale?;
    if let Some(v) = row.roc_pct_2026_actual_minor {
        return Some((v, scale));
    }
    if let Some(v) = row.roc_pct_2026_estimate_minor {
        return Some((v, scale));
    }
    if let Some(v) = row.roc_pct_2025_actual_minor {
        return Some((v, scale));
    }
    None
}

fn three_pay_yield_bps(
    hist: Option<&DeclarationHistoryRowBody>,
    last_price_minor: Option<i64>,
    freq: &str,
) -> Option<i64> {
    let price = last_price_minor.filter(|p| *p > 0)?;
    let periods = i64::from(periods_per_year(freq)?);
    let pays = &hist?.in_force_pays;
    let mut sum = 0i64;
    let mut count = 0usize;
    for pay in pays.iter().filter(|p| p.amount_per_share_minor.is_some()).take(3) {
        let amt = pay.amount_per_share_minor?;
        sum += scale_to_scale4(amt, pay.amount_scale);
        count += 1;
    }
    if count < 3 {
        return None;
    }
    let mean = sum / 3;
    let annual = mean * periods;
    // price is typically scale 2; convert to scale 4 for ratio
    let price4 = scale_to_scale4(price, 2);
    if price4 == 0 {
        return None;
    }
    Some((annual * 10_000) / price4)
}

fn plan_check_text(
    hist: Option<&DeclarationHistoryRowBody>,
    plan_known: bool,
    plan_minor: i64,
    plan_scale: u8,
) -> String {
    if !plan_known {
        return "unknown".into();
    }
    let Some(h) = hist else {
        return "unknown".into();
    };
    let plan4 = scale_to_scale4(plan_minor, plan_scale);
    let mut above = 0u32;
    let mut equal = 0u32;
    let mut below = 0u32;
    for pay in &h.in_force_pays {
        let Some(amt) = pay.amount_per_share_minor else {
            continue;
        };
        let a = scale_to_scale4(amt, pay.amount_scale);
        if a > plan4 {
            above += 1;
        } else if a < plan4 {
            below += 1;
        } else {
            equal += 1;
        }
    }
    let counted = above + equal + below;
    if counted == 0 {
        return "unknown".into();
    }
    format!("{above}↑ {equal}= {below}↓")
}

async fn trends_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let rows = home.weeks.iter().map(trend_line).collect();
    Ok(Sheet {
        name: "Trends".into(),
        headers: vec!["Week", "Profit", "Total cash", "Car balance"],
        rows,
    })
}

fn trend_line(point: &TrendsWeekPoint) -> Vec<String> {
    vec![
        point.period_end.clone(),
        money_text(point.profit_minor, 2),
        money_text(point.total_cash_minor, 2),
        money_unknown(point.car_balance_minor, 2),
    ]
}

async fn income_plan_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let week = crate::queries::income_plan_week_view(canonical, as_of.to_string(), as_of.to_string()).await?;
    let rows = week.positions.iter().map(income_line).collect();
    Ok(Sheet {
        name: "Income Plan".into(),
        headers: vec!["Symbol", "Plan $", "Decl $", "Actual"],
        rows,
    })
}

fn income_line(row: &IncomePlanPositionBody) -> Vec<String> {
    vec![
        row.symbol.clone(),
        known_money(row.plan_known, row.planned_minor, row.scale),
        if row.declaration_known {
            money_unknown(row.declaration_per_share_minor, row.declaration_per_share_scale)
        } else {
            "unknown".into()
        },
        known_money(row.actual_known, row.actual_minor, row.scale),
    ]
}

async fn market_impact_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::queries::market_impact_view(canonical).await?;
    let rows = body.rows.iter().map(impact_line).collect();
    Ok(Sheet {
        name: "Market impact planner".into(),
        headers: vec!["Symbol", "Bull total return", "Bear total return"],
        rows,
    })
}

fn impact_line(row: &MarketImpactRowBody) -> Vec<String> {
    vec![
        row.symbol.clone(),
        bps_text(row.bull_total_return_bps),
        bps_text(row.bear_total_return_bps),
    ]
}

async fn dashboard_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let home = home_values(canonical, as_of).await?;
    let rows = home
        .accounts
        .iter()
        .map(|series| {
            vec![
                series.account_name.clone(),
                money_unknown(series.current_minor, series.scale),
                home.as_of.clone(),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Dashboard".into(),
        headers: vec!["Label", "Value", "As of"],
        rows,
    })
}

async fn position_master_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::queries::position_master_view(canonical).await?;
    let rows = body
        .rows
        .iter()
        .map(|row: &PositionMasterRowBody| {
            let price_scale = row.last_price_scale.unwrap_or(2);
            vec![
                row.symbol.clone(),
                row.name.clone(),
                row.risk_tier.clone(),
                money_unknown(row.last_price_minor, price_scale),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Position Details".into(),
        headers: vec!["Symbol", "Name", "Risk", "Price"],
        rows,
    })
}

async fn holdings_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::queries::holdings_view(canonical).await?;
    let rows = body.lots.iter().map(holding_line).collect();
    Ok(Sheet {
        name: "Holdings".into(),
        headers: vec!["Account", "Symbol", "Opened", "Shares", "Performance cost", "Tax cost"],
        rows,
    })
}

fn holding_line(lot: &HoldingsLotBody) -> Vec<String> {
    vec![
        lot.account_name.clone(),
        lot.symbol.clone(),
        lot.opened_on.clone(),
        money_text(lot.remaining_quantity_minor, lot.quantity_scale),
        money_text(lot.remaining_performance_minor, lot.scale),
        money_text(lot.remaining_tax_minor, lot.scale),
    ]
}

async fn cart_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::cart::executed_list(canonical, None).await?;
    let rows = body
        .items
        .iter()
        .map(|row| {
            vec![
                row.account_name.clone(),
                row.name.clone(),
                row.as_of.clone(),
                money_text(row.non_cash_sales_minor, row.scale),
                money_text(row.realized_pl_minor, row.scale),
                money_text(row.invested_minor, row.scale),
                money_unknown(row.delta_annual_income_minor, row.scale),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Shopping Cart".into(),
        headers: vec![
            "Account",
            "Name",
            "As of",
            "Sales",
            "Realized P/L",
            "Invested",
            "Annual income change",
        ],
        rows,
    })
}

async fn cash_week_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let body = crate::cash_management::cash_management_week(canonical, as_of).await?;
    let rows = body.rows.iter().map(week_line).collect();
    Ok(Sheet {
        name: "Cash Management".into(),
        headers: vec!["Account", "Type", "Date", "Gross", "Federal", "State", "Net"],
        rows,
    })
}

fn week_line(row: &CashManagementWeekRow) -> Vec<String> {
    vec![
        row.account_name.clone(),
        row.activity_type.clone(),
        row.occurred_on.clone(),
        money_text(row.gross_minor, row.scale),
        money_text(row.federal_withholding_minor, row.scale),
        money_text(row.state_withholding_minor, row.scale),
        money_text(row.net_minor, row.scale),
    ]
}

async fn week_ahead_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let body = crate::week_ahead::week_ahead_get(canonical, as_of).await?;
    let rows = body.rows.iter().map(ahead_line).collect();
    Ok(Sheet {
        name: "Week ahead".into(),
        headers: vec!["Date", "Account", "Transaction", "Amount", "Note"],
        rows,
    })
}

fn ahead_line(row: &WeekAheadRow) -> Vec<String> {
    vec![
        row.occurred_on.clone(),
        row.account.clone(),
        row.transaction.clone(),
        money_text(row.amount_minor, row.scale),
        row.note.clone(),
    ]
}

async fn cash_ytd_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let body = crate::cash_ytd::cash_ytd_get(canonical, as_of, "account").await?;
    let rows = body.rows.iter().map(|row| ytd_line(row, body.scale)).collect();
    Ok(Sheet {
        name: "Cash YTD".into(),
        headers: vec!["Label", "Actual", "Remaining", "EOY"],
        rows,
    })
}

fn ytd_line(row: &CashYtdRow, scale: u8) -> Vec<String> {
    vec![
        row.label.clone(),
        money_text(row.actual_minor, scale),
        money_unknown(row.remaining_minor, scale),
        money_unknown(row.eoy_minor, scale),
    ]
}

async fn elements_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let body = crate::cash_register::cash_element_list_get(canonical, "all", as_of).await?;
    let rows = body.items.iter().map(element_line).collect();
    Ok(Sheet {
        name: "Elements".into(),
        headers: vec!["Account", "Kind", "Cadence", "Amount", "Note"],
        rows,
    })
}

fn element_line(row: &CashElementListItem) -> Vec<String> {
    vec![
        row.account.clone(),
        row.kind.clone(),
        row.cadence.clone(),
        money_text(row.amount_minor, 2),
        row.note.clone(),
    ]
}

async fn coverage_sheet(canonical: &dyn Canonical, as_of: &str) -> Result<Sheet, PlatformError> {
    let body = crate::cash_coverage::cash_coverage_get(canonical, as_of, "year").await?;
    let rows = body.rows.iter().map(|row| coverage_line(row, body.scale)).collect();
    Ok(Sheet {
        name: "Income vs Expense".into(),
        headers: vec!["Account", "Plan income", "Plan expense", "Actual"],
        rows,
    })
}

fn coverage_line(row: &CashCoverageRow, scale: u8) -> Vec<String> {
    vec![
        row.account.clone(),
        money_unknown(row.plan_income_minor, scale),
        money_unknown(row.plan_expense_minor, scale),
        money_unknown(row.actual_minor, scale),
    ]
}

async fn register_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = canonical.external_register_get(None).await?;
    let rows = body.lines.iter().map(register_line).collect();
    Ok(Sheet {
        name: "External accounts".into(),
        headers: vec!["Pay type", "Date", "Total spent", "Category", "Vendor", "Description"],
        rows,
    })
}

fn register_line(row: &ExternalRegisterLine) -> Vec<String> {
    vec![
        row.pay_type.clone(),
        row.occurred_on.clone().unwrap_or_default(),
        money_text(row.amount_minor, row.scale),
        row.category.clone(),
        row.vendor.clone(),
        row.description.clone(),
    ]
}

async fn debt_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = canonical.external_account_manager_get().await?;
    let rows = body.accounts.iter().map(debt_line).collect();
    Ok(Sheet {
        name: "Debt planner".into(),
        headers: vec![
            "Account",
            "Starting balance",
            "Current balance",
            "Interest rate",
            "Frequency",
            "Due day",
            "Payment",
        ],
        rows,
    })
}

async fn accounts_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let rows = canonical
        .account_list()
        .await?
        .into_iter()
        .map(|row| {
            vec![
                row.name,
                row.kind,
                row.cash_symbol,
                row.broker_account_number,
                money_unknown(row.min_balance_target_minor, 2),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Account Management".into(),
        headers: vec![
            "Account",
            "Kind",
            "Cash symbol",
            "Broker account number",
            "Min balance target",
        ],
        rows,
    })
}

fn debt_line(row: &ExternalManagedAccount) -> Vec<String> {
    let apr = match row.apr_ppm {
        None => "unknown".into(),
        Some(ppm) => format!("{}.{:04}%", ppm / 10_000, (ppm.abs() % 10_000)),
    };
    vec![
        row.name.clone(),
        money_unknown(row.starting_minor, 2),
        money_unknown(row.current_minor, 2),
        apr,
        row.frequency.clone().unwrap_or_default(),
        row.due_on.clone().unwrap_or_default(),
        money_unknown(row.payment_minor, 2),
    ]
}

async fn securities_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let rows = canonical
        .security_list()
        .await?
        .into_iter()
        .map(|row| vec![row.symbol, row.name])
        .collect();
    Ok(Sheet {
        name: "Securities".into(),
        headers: vec!["Symbol", "Name"],
        rows,
    })
}

async fn contracts_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::option_contract::contract_list(canonical, None).await?;
    let rows = body
        .items
        .iter()
        .map(|row| {
            vec![
                row.occ_symbol.clone(),
                row.underlying.clone(),
                row.expiry_on.clone(),
                row.put_call.clone(),
                money_text(row.strike_minor, row.scale),
                row.status.clone(),
                money_unknown(row.underlying_last_minor, row.scale),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Contract positions".into(),
        headers: vec!["Contract", "Underlying", "Expiry", "Put or call", "Strike", "Status", "Underlying last"],
        rows,
    })
}

async fn tasks_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let body = crate::task::task_list(canonical, None, None).await?;
    let rows = body
        .items
        .iter()
        .map(|row| {
            vec![
                row.title.clone(),
                row.status.clone(),
                row.domain.clone(),
                row.due_on.clone(),
            ]
        })
        .collect();
    Ok(Sheet {
        name: "Tasks".into(),
        headers: vec!["Title", "Status", "Domain", "Due"],
        rows,
    })
}

async fn import_sheet(canonical: &dyn Canonical) -> Result<Sheet, PlatformError> {
    let batch = canonical.import_pending_get().await?;
    let rows = match batch {
        None => Vec::new(),
        Some(batch) => vec![vec![batch.filename, batch.status]],
    };
    Ok(Sheet {
        name: "Import".into(),
        headers: vec!["File", "Status"],
        rows,
    })
}

fn interest_sheet() -> Sheet {
    let headers = vec![
        "Period",
        "Period return %",
        "Daily",
        "Weekly",
        "Monthly",
        "Annual",
    ];
    let rows = ["Daily", "Weekly", "Monthly", "Quarterly", "Annual"]
        .into_iter()
        .map(|period| vec![period.into(), String::new(), "—".into(), "—".into(), "—".into(), "—".into()])
        .collect();
    Sheet {
        name: "Interest rate calculator".into(),
        headers,
        rows,
    }
}

fn field_intent_sheet() -> Sheet {
    Sheet {
        name: "Calculator columns".into(),
        headers: vec!["Column", "Kind", "Intent", "Formula", "Status"],
        rows: field_intent_rows(),
    }
}

fn field_intent_rows() -> Vec<Vec<String>> {
    const SOURCE: &str = include_str!("../../../apps/desktop/src/features/field-intent/calculatorColumns.ts");
    let mut rows = Vec::new();
    let mut name = String::new();
    let mut kind = String::new();
    let mut intent = String::new();
    let mut formula = String::new();
    let mut status = String::new();
    let mut seen = false;
    let mut pending = "";
    let flush = |rows: &mut Vec<Vec<String>>,
                 seen: &mut bool,
                 name: &mut String,
                 kind: &mut String,
                 intent: &mut String,
                 formula: &mut String,
                 status: &mut String| {
        if *seen {
            rows.push(vec![
                name.clone(),
                kind.clone(),
                intent.clone(),
                formula.clone(),
                status.clone(),
            ]);
        }
        *seen = false;
        name.clear();
        kind.clear();
        intent.clear();
        formula.clear();
        status.clear();
    };
    for line in SOURCE.lines() {
        let line = line.trim();
        if let Some(value) = quoted_field(line, "name") {
            flush(
                &mut rows,
                &mut seen,
                &mut name,
                &mut kind,
                &mut intent,
                &mut formula,
                &mut status,
            );
            name = value;
            seen = true;
            pending = "";
        } else if let Some(value) = quoted_field(line, "kind") {
            kind = value;
            pending = "";
        } else if let Some(value) = quoted_field(line, "intent") {
            intent = value;
            pending = "";
        } else if line == "intent:" {
            pending = "intent";
        } else if let Some(value) = quoted_field(line, "formula") {
            formula = value;
            pending = "";
        } else if line == "formula:" {
            pending = "formula";
        } else if let Some(value) = quoted_field(line, "status") {
            status = value;
            pending = "";
        } else if let Some(value) = bare_quoted(line) {
            if pending == "intent" {
                intent = value;
            } else if pending == "formula" {
                formula = value;
            }
            pending = "";
        }
    }
    flush(
        &mut rows,
        &mut seen,
        &mut name,
        &mut kind,
        &mut intent,
        &mut formula,
        &mut status,
    );
    rows
}

fn bare_quoted(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches(',');
    let rest = line.strip_prefix('"')?.strip_suffix('"')?;
    Some(rest.to_string())
}

fn quoted_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("{key}: \"");
    let rest = line.trim().strip_prefix(needle.as_str())?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn catalog_sheet(module_id: &str) -> Sheet {
    let Ok(catalog) = crate::core_functions::core_functions_catalog() else {
        return Sheet {
            name: "Component Registry".into(),
            headers: vec!["Module", "Status", "Folder", "Menu areas", "Core functions", "Host", "Description"],
            rows: Vec::new(),
        };
    };
    if module_id == "settings" {
        let rows = catalog
            .items
            .iter()
            .map(|item| vec![item.function.clone(), item.menu_area.clone()])
            .collect();
        return Sheet {
            name: "Settings".into(),
            headers: vec!["Function", "Menu area"],
            rows,
        };
    }
    if module_id == "shell" {
        let rows = catalog
            .modules
            .iter()
            .find(|m| m.id == "shell")
            .map(|m| {
                m.core_function_ids
                    .iter()
                    .map(|id| vec![id.clone()])
                    .collect()
            })
            .unwrap_or_default();
        return Sheet {
            name: "Shell".into(),
            headers: vec!["Function"],
            rows,
        };
    }
    let rows = catalog
        .modules
        .iter()
        .map(|module| {
            let description = module.description.clone();
            vec![
                module.title.clone(),
                module.status.clone(),
                module.folder.clone(),
                module.menu_areas.join(", "),
                module.core_function_ids.join(", "),
                module.host.clone(),
                description,
            ]
        })
        .collect();
    Sheet {
        name: "Component Registry".into(),
        headers: vec!["Module", "Status", "Folder", "Menu areas", "Core functions", "Host", "Description"],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{export_covers, field_intent_rows, interest_sheet, money_text, money_unknown};

    #[test]
    fn missing_money_is_unknown_and_zero_stays_zero() {
        assert_eq!(money_unknown(None, 2), "unknown");
        assert_eq!(money_unknown(Some(0), 2), "0.00");
        assert_eq!(money_text(3200, 2), "32.00");
    }

    #[test]
    fn interest_export_leaves_the_return_blank() {
        let sheet = interest_sheet();
        assert_eq!(sheet.rows[0][1], "");
        assert_eq!(sheet.rows[0][2], "—");
        assert!(sheet.rows.iter().all(|row| !row.iter().any(|cell| cell == "0")));
    }

    #[test]
    fn field_intent_reads_the_column_contract() {
        let rows = field_intent_rows();
        assert!(rows.iter().any(|row| row[0] == "Symbol" && !row[1].is_empty()));
    }

    #[test]
    fn every_shipped_module_has_an_arm() {
        for id in [
            "graphing",
            "new-investment-readiness",
            "shopping-cart",
            "cash-management",
            "cash-week-desk",
            "week-ahead",
            "household-income",
            "magi-forecast",
            "cash-elements",
            "cash-management-ytd",
            "cm-element-management",
            "cm-cashflow-manager",
            "cm-weekly-updates",
            "cm-car-account-tax",
            "cm-coverage",
            "cm-external",
            "cm-debt-planner",
            "account-management",
            "home",
            "income-plan",
            "calculator",
            "market-impact",
            "dashboard",
            "trends",
            "position-details",
            "holdings",
            "add-position",
            "add-lot",
            "interest-rate",
            "contract-positions",
            "roadmap",
            "field-intent",
            "task-manager",
            "reevaluate-collector",
            "tickets",
            "collectors",
            "import",
            "lots",
            "settings",
            "components",
            "screen-atlas",
            "shell",
        ] {
            assert!(export_covers(id, ""), "{id}");
        }
        assert!(export_covers("graphing", "live-by-risk"));
        assert!(export_covers("graphing", "account-values"));
        assert!(export_covers("home", "dividend-plan"));
        assert!(export_covers("calculator", "calculator-sheet"));
        assert!(export_covers("trends", "trends-weeks"));
        assert!(export_covers("cm-external", "cct-register"));
        assert!(export_covers("cm-debt-planner", "debt-accounts"));
        assert!(export_covers("cm-debt-planner", "debt-buckets"));
        assert!(export_covers("field-intent", "calculator-columns"));
        assert!(export_covers("contract-positions", "contract-create"));
        assert!(export_covers("contract-positions", "contract-open"));
        assert!(export_covers("contract-positions", "contract-closed"));
        assert!(export_covers("roadmap", "cash-covered-puts"));
    }

    #[test]
    fn page_export_is_one_workbook_with_two_sheets() {
        use super::{unique_sheet_names, workbook_bytes, Sheet};
        let sheets = vec![
            Sheet {
                name: "Portfolio summary".into(),
                headers: vec!["Label", "Value"],
                rows: vec![],
            },
            Sheet {
                name: "Dividend Plan".into(),
                headers: vec!["Account", "Plan"],
                rows: vec![],
            },
        ];
        let bytes = workbook_bytes(&sheets).expect("workbook");
        let has = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"sheet1.xml"), "first component sheet");
        assert!(has(b"sheet2.xml"), "second component sheet");
        assert!(!has(b"sheet3.xml"), "two components are two sheets");
        let repeated = vec![
            Sheet {
                name: "Income".into(),
                headers: vec!["Week"],
                rows: vec![],
            },
            Sheet {
                name: "Income".into(),
                headers: vec!["Week"],
                rows: vec![],
            },
        ];
        assert_eq!(
            unique_sheet_names(&repeated),
            vec!["Income".to_string(), "Income 2".to_string()]
        );
        assert_eq!(super::page_file_name("Home"), "Home.xlsx");
    }
}
