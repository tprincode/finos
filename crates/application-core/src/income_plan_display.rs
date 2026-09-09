//! Pattern A/B snapshot assembly and Income Plan Print / Export.
//! Print does not load or iframe the companion HTML mock.

use std::collections::{BTreeMap, HashSet};

use chrono::Datelike;
use rust_xlsxwriter::Workbook;

use crate::contracts::{
    IncomePlanCoverBody, IncomePlanExportBody, IncomePlanGridBody, IncomePlanGridWeekMeta,
    IncomePlanMoneyCell, IncomePlanTable1Row, IncomePlanTable2Group, IncomePlanTable2Row,
    IncomePlanWeekBody,
};
use financial_domain::income_plan::{
    account_key_from_display, actuals_known, cadence_group, declaration_is_current,
    delta_to_plan_pct_minor, display_account_label, table2_cell_tone, GridWeekKind,
    DEFAULT_ACCOUNTS_ON, TABLE1_ACCOUNT_ORDER,
};

const HTML_MOCK: &str = "Income_Plan_Pattern_A_B_Simulation.html";

pub fn default_selected_accounts() -> Vec<String> {
    DEFAULT_ACCOUNTS_ON.iter().map(|s| (*s).to_string()).collect()
}

pub fn normalize_selected_accounts(raw: &[String]) -> Vec<String> {
    if raw.is_empty() {
        return default_selected_accounts();
    }
    raw.iter()
        .filter_map(|n| account_key_from_display(n).map(display_account_label))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn take_newer_declaration(
    entry: &mut IncomePlanTable2Row,
    pos: &crate::contracts::IncomePlanPositionBody,
) {
    if pos.declaration_per_share_minor.is_none() {
        return;
    }
    let incoming = pos.declaration_entered_on.as_deref().unwrap_or("");
    let existing = entry.declaration_entered_on.as_deref().unwrap_or("");
    if entry.declaration_per_share_minor.is_none() || incoming > existing {
        entry.declaration_per_share_minor = pos.declaration_per_share_minor;
        entry.declaration_per_share_scale = pos.declaration_per_share_scale;
        entry.declaration_entered_on = pos.declaration_entered_on.clone();
    }
}

fn week_positions_by_cadence(
    week: &IncomePlanWeekBody,
) -> Vec<(&'static str, Vec<&crate::contracts::IncomePlanPositionBody>)> {
    ["Monthly", "Quarterly", "Weekly", "Other"]
        .into_iter()
        .filter_map(|name| {
            let rows: Vec<_> = week
                .positions
                .iter()
                .filter(|pos| cadence_group(&pos.cadence) == name)
                .collect();
            (!rows.is_empty()).then_some((name, rows))
        })
        .collect()
}

fn fmt_per_share(amt: Option<i64>, scale: u8) -> String {
    let Some(minor) = amt else {
        return String::new();
    };
    let places = scale as usize;
    let sign = if minor < 0 { "-" } else { "" };
    let digits = minor.unsigned_abs().to_string();
    if places == 0 {
        return format!("{sign}${digits}");
    }
    let pad = format!("{digits:0>width$}", width = places + 1);
    let split = pad.len() - places;
    format!("{sign}${}.{}", &pad[..split], &pad[split..])
}

fn selected_keys(accounts: &[String]) -> HashSet<String> {
    accounts
        .iter()
        .filter_map(|n| account_key_from_display(n).map(|k| k.to_string()))
        .collect()
}

fn kind_label(kind: GridWeekKind) -> &'static str {
    match kind {
        GridWeekKind::Closed => "closed",
        GridWeekKind::InProgress => "in_progress",
        GridWeekKind::Future => "future",
    }
}

fn line_for_display<'a>(
    week: &'a IncomePlanWeekBody,
    display: &str,
) -> Option<&'a crate::contracts::IncomePlanLineBody> {
    week.lines.iter().find(|l| {
        l.account_name == display
            || account_key_from_display(&l.account_name)
                .map(display_account_label)
                .as_deref()
                == Some(display)
    })
}

fn position_amounts(
    pos: &crate::contracts::IncomePlanPositionBody,
    keys: &HashSet<String>,
) -> (Option<i64>, Option<i64>, Option<i64>, bool) {
    if pos.accounts.is_empty() {
        return (
            pos.plan_known.then_some(pos.planned_minor),
            pos.actual_known.then_some(pos.actual_minor),
            pos.declaration_known.then_some(pos.declaration_minor),
            pos.plan_known,
        );
    }
    let slices: Vec<_> = pos
        .accounts
        .iter()
        .filter(|a| {
            account_key_from_display(&a.account_name)
                .map(|k| keys.contains(k))
                .unwrap_or(false)
        })
        .collect();
    if slices.is_empty() {
        return (None, None, None, false);
    }
    let plan_known = slices.iter().all(|a| a.plan_known);
    let planned = slices.iter().map(|a| a.planned_minor).sum::<i64>();
    let actual_known = slices.iter().any(|a| a.actual_known);
    let actual = slices.iter().map(|a| a.actual_minor).sum::<i64>();
    let decl_known = slices.iter().any(|a| a.declaration_known);
    let decl = slices.iter().map(|a| a.declaration_minor).sum::<i64>();
    (
        plan_known.then_some(planned),
        actual_known.then_some(actual),
        decl_known.then_some(decl),
        plan_known,
    )
}

pub fn assemble_grid(
    as_of_date: String,
    historical_weeks: u32,
    future_weeks: u32,
    selected_accounts: Vec<String>,
    selected_week_end: String,
    weeks: &[(GridWeekKind, IncomePlanWeekBody)],
    year_weeks: &[(GridWeekKind, IncomePlanWeekBody)],
) -> IncomePlanGridBody {
    let selected = if selected_accounts.is_empty() {
        default_selected_accounts()
    } else {
        selected_accounts
    };
    let keys = selected_keys(&selected);
    let mut account_rows: Vec<String> = TABLE1_ACCOUNT_ORDER
        .iter()
        .map(|s| (*s).to_string())
        .filter(|n| selected.iter().any(|s| s == n))
        .collect();
    for extra in ["Speculation", "Energy", "Robinhood"] {
        if selected.iter().any(|s| s == extra) && !account_rows.iter().any(|s| s == extra) {
            account_rows.push(extra.to_string());
        }
    }

    let visible: Vec<(GridWeekKind, &IncomePlanWeekBody)> = weeks
        .iter()
        .map(|(k, b)| (*k, b))
        .collect();
    let week_ends: Vec<String> = visible.iter().map(|(_, b)| b.end.clone()).collect();
    let year_label = as_of_date
        .get(..4)
        .unwrap_or("2026")
        .to_string();
    let mut table1_columns = vec!["label".into(), year_label.clone()];
    table1_columns.extend(week_ends.iter().cloned());

    let plan_for = |week: &IncomePlanWeekBody, display: &str| -> i64 {
        line_for_display(week, display)
            .filter(|l| l.plan_known)
            .map(|l| l.planned_minor)
            .unwrap_or(0)
    };
    let actual_for = |week: &IncomePlanWeekBody, display: &str, kind: GridWeekKind| -> Option<i64> {
        if !actuals_known(kind) {
            return None;
        }
        line_for_display(week, display).map(|l| l.actual_minor)
    };

    let mut table1 = Vec::new();
    let total_plan_cells: Vec<IncomePlanMoneyCell> = visible
        .iter()
        .map(|(kind, week)| {
            let amt: i64 = account_rows.iter().map(|a| plan_for(week, a)).sum();
            IncomePlanMoneyCell {
                week_end: week.end.clone(),
                amount_minor: Some(amt),
                tone: if matches!(kind, GridWeekKind::Closed | GridWeekKind::InProgress) {
                    "total_plan".into()
                } else {
                    "future".into()
                },
            }
        })
        .collect();
    let total_actual_cells: Vec<IncomePlanMoneyCell> = visible
        .iter()
        .map(|(kind, week)| IncomePlanMoneyCell {
            week_end: week.end.clone(),
            amount_minor: if actuals_known(*kind) {
                Some(account_rows.iter().filter_map(|a| actual_for(week, a, *kind)).sum())
            } else {
                None
            },
            tone: "total_actual".into(),
        })
        .collect();
    let total_diff_cells: Vec<IncomePlanMoneyCell> = total_plan_cells
        .iter()
        .zip(total_actual_cells.iter())
        .map(|(p, a)| IncomePlanMoneyCell {
            week_end: p.week_end.clone(),
            amount_minor: match (p.amount_minor, a.amount_minor) {
                (Some(plan), Some(act)) => Some(act - plan),
                _ => None,
            },
            tone: String::new(),
        })
        .collect();
    let delta_cells: Vec<IncomePlanMoneyCell> = total_plan_cells
        .iter()
        .zip(total_actual_cells.iter())
        .map(|(p, a)| IncomePlanMoneyCell {
            week_end: p.week_end.clone(),
            amount_minor: None,
            tone: match (p.amount_minor, a.amount_minor) {
                (Some(plan), Some(act)) => delta_to_plan_pct_minor(act, plan != 0, plan)
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            },
        })
        .collect();

    let year_plan: i64 = year_weeks
        .iter()
        .map(|(_, w)| account_rows.iter().map(|a| plan_for(w, a)).sum::<i64>())
        .sum();
    let year_actual: Option<i64> = {
        let vals: Vec<i64> = year_weeks
            .iter()
            .filter(|(k, _)| actuals_known(*k))
            .map(|(_, w)| {
                account_rows
                    .iter()
                    .filter_map(|a| actual_for(w, a, GridWeekKind::Closed))
                    .sum::<i64>()
            })
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.into_iter().sum())
        }
    };
    let year_diff = year_actual.map(|a| a - year_plan);
    let year_delta = year_actual.and_then(|a| delta_to_plan_pct_minor(a, year_plan != 0, year_plan));

    table1.push(IncomePlanTable1Row {
        id: "delta_to_plan_pct".into(),
        label: "Delta to plan %".into(),
        year_minor: year_delta,
        year_pct_minor: year_delta,
        cells: delta_cells,
    });
    table1.push(IncomePlanTable1Row {
        id: "total_plan".into(),
        label: "Total Plan".into(),
        year_minor: Some(year_plan),
        year_pct_minor: None,
        cells: total_plan_cells.clone(),
    });
    for acct in &account_rows {
        let cells: Vec<IncomePlanMoneyCell> = visible
            .iter()
            .map(|(_, week)| IncomePlanMoneyCell {
                week_end: week.end.clone(),
                amount_minor: Some(plan_for(week, acct)),
                tone: String::new(),
            })
            .collect();
        let year_avg = if year_weeks.is_empty() {
            None
        } else {
            Some(
                year_weeks
                    .iter()
                    .map(|(_, w)| plan_for(w, acct))
                    .sum::<i64>()
                    / year_weeks.len() as i64,
            )
        };
        table1.push(IncomePlanTable1Row {
            id: format!("plan_{acct}"),
            label: acct.clone(),
            year_minor: year_avg,
            year_pct_minor: None,
            cells,
        });
    }
    table1.push(IncomePlanTable1Row {
        id: "total_actual".into(),
        label: "Total Actual".into(),
        year_minor: year_actual,
        year_pct_minor: None,
        cells: total_actual_cells.clone(),
    });
    for acct in &account_rows {
        let cells: Vec<IncomePlanMoneyCell> = visible
            .iter()
            .map(|(kind, week)| IncomePlanMoneyCell {
                week_end: week.end.clone(),
                amount_minor: actual_for(week, acct, *kind),
                tone: String::new(),
            })
            .collect();
        let closed: Vec<i64> = year_weeks
            .iter()
            .filter(|(k, _)| actuals_known(*k))
            .filter_map(|(_, w)| actual_for(w, acct, GridWeekKind::Closed))
            .collect();
        let year_avg = if closed.is_empty() {
            None
        } else {
            Some(closed.iter().sum::<i64>() / closed.len() as i64)
        };
        table1.push(IncomePlanTable1Row {
            id: format!("actual_{acct}"),
            label: acct.clone(),
            year_minor: year_avg,
            year_pct_minor: None,
            cells,
        });
    }
    table1.push(IncomePlanTable1Row {
        id: "total_difference".into(),
        label: "Total Difference".into(),
        year_minor: year_diff,
        year_pct_minor: None,
        cells: total_diff_cells,
    });

    let mut by_symbol: BTreeMap<String, IncomePlanTable2Row> = BTreeMap::new();
    for (kind, week) in &visible {
        for pos in &week.positions {
            if cadence_group(&pos.cadence) == "Other" && pos.cadence.eq_ignore_ascii_case("None") {
                continue;
            }
            let (planned, actual, _, plan_known) = position_amounts(pos, &keys);
            if !plan_known && actual.is_none() && planned.unwrap_or(0) == 0 && pos.accounts.iter().all(|a| {
                !account_key_from_display(&a.account_name)
                    .map(|k| keys.contains(k))
                    .unwrap_or(false)
            }) {
                continue;
            }
            if pos.accounts.iter().any(|a| {
                account_key_from_display(&a.account_name)
                    .map(|k| keys.contains(k))
                    .unwrap_or(false)
            }) || pos.accounts.is_empty()
            {
                let entry = by_symbol.entry(pos.symbol.clone()).or_insert_with(|| {
                    IncomePlanTable2Row {
                        symbol: pos.symbol.clone(),
                        cadence: pos.cadence.clone(),
                        last_update: pos.last_update.clone(),
                        declaration_per_share_minor: None,
                        declaration_per_share_scale: 0,
                        declaration_entered_on: None,
                        declaration_current: false,
                        cells: week_ends
                            .iter()
                            .map(|end| IncomePlanMoneyCell {
                                week_end: end.clone(),
                                amount_minor: None,
                                tone: "empty".into(),
                            })
                            .collect(),
                    }
                });
                if entry.last_update.is_none() {
                    entry.last_update = pos.last_update.clone();
                }
                take_newer_declaration(entry, pos);
                if let Some(cell) = entry.cells.iter_mut().find(|c| c.week_end == week.end) {
                    let tone = table2_cell_tone(planned, actual, *kind);
                    cell.amount_minor = planned.filter(|p| *p != 0).or(planned);
                    if planned.unwrap_or(0) == 0 {
                        cell.amount_minor = None;
                    }
                    cell.tone = match tone {
                        financial_domain::income_plan::Table2Tone::Empty => "empty",
                        financial_domain::income_plan::Table2Tone::Future => "future",
                        financial_domain::income_plan::Table2Tone::Ok => "ok",
                        financial_domain::income_plan::Table2Tone::Variance => "variance",
                        financial_domain::income_plan::Table2Tone::Miss => "miss",
                    }
                    .into();
                }
            }
        }
    }

    let mut groups: BTreeMap<&str, Vec<IncomePlanTable2Row>> = BTreeMap::new();
    for mut row in by_symbol.into_values() {
        row.declaration_current = row.declaration_per_share_minor.is_some()
            && declaration_is_current(
                row.declaration_entered_on.as_deref().unwrap_or(""),
                &as_of_date,
            );
        groups
            .entry(cadence_group(&row.cadence))
            .or_default()
            .push(row);
    }
    let group_order = ["Monthly", "Quarterly", "Weekly", "Other"];
    let table2 = group_order
        .iter()
        .filter_map(|g| {
            groups.remove(g).map(|mut rows| {
                rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                IncomePlanTable2Group {
                    cadence: (*g).to_string(),
                    rows,
                }
            })
        })
        .collect();

    IncomePlanGridBody {
        as_of_date,
        historical_weeks,
        future_weeks,
        selected_accounts: selected,
        table1_columns,
        weeks: visible
            .iter()
            .map(|(kind, week)| IncomePlanGridWeekMeta {
                start: week.start.clone(),
                end: week.end.clone(),
                kind: kind_label(*kind).to_string(),
            })
            .collect(),
        table1,
        table2,
        selected_week_end,
        scale: 2,
    }
}

pub fn table1_has_last_update(grid: &IncomePlanGridBody) -> bool {
    grid.table1_columns
        .iter()
        .any(|c| c.eq_ignore_ascii_case("last_update") || c.eq_ignore_ascii_case("Last Update"))
        || grid.table1.iter().any(|r| {
            r.id.eq_ignore_ascii_case("last_update") || r.label.eq_ignore_ascii_case("Last Update")
        })
}

pub fn table2_banned_columns(grid: &IncomePlanGridBody) -> bool {
    grid.table2.iter().any(|g| {
        g.rows.iter().any(|r| {
            r.symbol.eq_ignore_ascii_case("quantity")
                || r.cadence.eq_ignore_ascii_case("ytd_total")
                || r.cadence.eq_ignore_ascii_case("annual_pct")
        })
    })
}

pub struct ExportRequest<'a> {
    pub pattern: &'a str,
    pub format: &'a str,
    pub printed_at: &'a str,
    pub grid: Option<&'a IncomePlanGridBody>,
    pub week: Option<&'a IncomePlanWeekBody>,
    pub selected_accounts: &'a [String],
    pub historical_weeks: u32,
    pub future_weeks: u32,
}

pub fn build_export(req: ExportRequest<'_>) -> Result<IncomePlanExportBody, String> {
    let pattern = req.pattern.to_ascii_uppercase();
    let week_ending = if pattern == "B" {
        req.week.map(|w| w.end.clone())
    } else {
        None
    };
    let cover = IncomePlanCoverBody {
        pattern: pattern.clone(),
        printed_at: req.printed_at.to_string(),
        selected_accounts: req.selected_accounts.to_vec(),
        historical_weeks: req.historical_weeks,
        future_weeks: req.future_weeks,
        week_ending,
    };
    let print_html = print_html(&cover, req.grid, req.week);
    if print_html.contains(HTML_MOCK) {
        return Err("print must not invoke the companion HTML".into());
    }
    let landscape = pattern == "A";
    let format = req.format.to_ascii_lowercase();
    let sheet_names = if pattern == "A" {
        vec!["AccountRollup".into(), "PositionGrid".into(), "Cover".into()]
    } else {
        vec!["WeekDetail".into(), "Cover".into()]
    };
    let (bytes, default_file_name) = match format.as_str() {
        "xlsx" | "excel" => (xlsx_bytes(&cover, req.grid, req.week)?, {
            format!("income-plan-{}-{}.xlsx", pattern.to_ascii_lowercase(), req.printed_at.replace(':', ""))
        }),
        "html" | "printdocument" => (print_html.as_bytes().to_vec(), {
            format!("income-plan-{}-{}.html", pattern.to_ascii_lowercase(), req.printed_at.replace(':', ""))
        }),
        _ => (pdf_bytes(&cover, req.grid, req.week, landscape), {
            format!("income-plan-{}-{}.pdf", pattern.to_ascii_lowercase(), req.printed_at.replace(':', ""))
        }),
    };
    Ok(IncomePlanExportBody {
        format: if format == "xlsx" || format == "excel" {
            "xlsx".into()
        } else if format == "html" || format == "printdocument" {
            "printHtml".into()
        } else {
            "pdf".into()
        },
        destination_kind: "local".into(),
        default_file_name,
        landscape,
        bytes_base64: b64(&bytes),
        print_html,
        cover,
        sheet_names,
    })
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn cover_lines(cover: &IncomePlanCoverBody) -> Vec<String> {
    let mut lines = vec![
        format!("pattern: {}", cover.pattern),
        format!("printed-at: {}", cover.printed_at),
        format!("selected accounts: {}", cover.selected_accounts.join(", ")),
        format!(
            "week window: historical {} future {}",
            cover.historical_weeks, cover.future_weeks
        ),
    ];
    if let Some(end) = &cover.week_ending {
        lines.push(format!("week-ending: {end}"));
    }
    lines
}

fn fmt_cell(amt: Option<i64>) -> String {
    match amt {
        None => String::new(),
        Some(v) => format!("{:.2}", v as f64 / 100.0),
    }
}

fn print_html(
    cover: &IncomePlanCoverBody,
    grid: Option<&IncomePlanGridBody>,
    week: Option<&IncomePlanWeekBody>,
) -> String {
    let mut html = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Income Plan</title></head><body>",
    );
    html.push_str("<h1>Income Plan Print</h1><pre>");
    for line in cover_lines(cover) {
        html.push_str(&escape(&line));
        html.push('\n');
    }
    html.push_str("</pre>");
    if cover.pattern == "A" {
        if let Some(grid) = grid {
            html.push_str("<h2>AccountRollup</h2><table>");
            html.push_str("<tr>");
            for c in &grid.table1_columns {
                html.push_str(&format!("<th>{}</th>", escape(c)));
            }
            html.push_str("</tr>");
            for row in &grid.table1 {
                html.push_str("<tr>");
                html.push_str(&format!("<td>{}</td>", escape(&row.label)));
                html.push_str(&format!("<td>{}</td>", escape(&fmt_opt(row.year_minor, row.year_pct_minor))));
                for cell in &row.cells {
                    let text = if row.id == "delta_to_plan_pct" {
                        cell.tone.clone()
                    } else {
                        fmt_cell(cell.amount_minor)
                    };
                    html.push_str(&format!(
                        "<td data-row=\"{}\" data-week-end=\"{}\">{}</td>",
                        escape(&row.id),
                        escape(&cell.week_end),
                        escape(&text)
                    ));
                }
                html.push_str("</tr>");
            }
            html.push_str("</table><h2>PositionGrid</h2><table>");
            html.push_str("<tr><th>symbol</th><th>last_update</th><th>decl_per_share</th>");
            if let Some(g) = grid.table2.first() {
                if let Some(r) = g.rows.first() {
                    for cell in &r.cells {
                        html.push_str(&format!("<th>{}</th>", escape(&cell.week_end)));
                    }
                }
            }
            html.push_str("</tr>");
            for group in &grid.table2 {
                html.push_str(&format!(
                    "<tr><th colspan=\"3\">{}</th></tr>",
                    escape(&group.cadence)
                ));
                for row in &group.rows {
                    html.push_str("<tr>");
                    html.push_str(&format!("<td>{}</td>", escape(&row.symbol)));
                    html.push_str(&format!(
                        "<td>{}</td>",
                        escape(row.last_update.as_deref().unwrap_or(""))
                    ));
                    html.push_str(&format!(
                        "<td>{}</td>",
                        escape(&fmt_per_share(
                            row.declaration_per_share_minor,
                            row.declaration_per_share_scale
                        ))
                    ));
                    for cell in &row.cells {
                        html.push_str(&format!("<td>{}</td>", escape(&fmt_cell(cell.amount_minor))));
                    }
                    html.push_str("</tr>");
                }
            }
            html.push_str("</table>");
        }
    } else if let Some(week) = week {
        html.push_str("<h2>WeekDetail</h2><table>");
        html.push_str(
            "<tr><th>symbol</th><th>last_update</th><th>decl_per_share</th><th>Plan $</th><th>Declaration $</th><th>Actual $</th><th>Variance</th></tr>",
        );
        for (group_name, rows) in week_positions_by_cadence(week) {
            html.push_str(&format!(
                "<tr><th colspan=\"7\">{}</th></tr>",
                escape(group_name)
            ));
            for pos in rows {
                let var = if pos.plan_known && pos.actual_known {
                    fmt_cell(Some(pos.actual_minor - pos.planned_minor))
                } else {
                    String::new()
                };
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape(&pos.symbol),
                    escape(pos.last_update.as_deref().unwrap_or("")),
                    escape(&fmt_per_share(pos.declaration_per_share_minor, pos.declaration_per_share_scale)),
                    escape(&fmt_cell(pos.plan_known.then_some(pos.planned_minor))),
                    escape(&fmt_cell(pos.declaration_known.then_some(pos.declaration_minor))),
                    escape(&fmt_cell(pos.actual_known.then_some(pos.actual_minor))),
                    escape(&var),
                ));
            }
        }
        html.push_str("</table>");
    }
    html.push_str("</body></html>");
    html
}

fn fmt_opt(year_minor: Option<i64>, year_pct: Option<i64>) -> String {
    if let Some(pct) = year_pct {
        format!("{:.2}%", pct as f64 / 100.0)
    } else {
        fmt_cell(year_minor)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn pdf_bytes(
    cover: &IncomePlanCoverBody,
    grid: Option<&IncomePlanGridBody>,
    week: Option<&IncomePlanWeekBody>,
    landscape: bool,
) -> Vec<u8> {
    let mut text = cover_lines(cover).join("\n");
    text.push('\n');
    if cover.pattern == "A" {
        text.push_str("AccountRollup\n");
        if let Some(grid) = grid {
            text.push_str(&format!("columns: {}\n", grid.table1_columns.join(" | ")));
            for row in &grid.table1 {
                let mut line = format!("{} | {}", row.label, fmt_opt(row.year_minor, row.year_pct_minor));
                for cell in &row.cells {
                    if row.id == "delta_to_plan_pct" {
                        line.push_str(" | ");
                        line.push_str(&cell.tone);
                    } else {
                        line.push_str(" | ");
                        line.push_str(&fmt_cell(cell.amount_minor));
                    }
                }
                text.push_str(&line);
                text.push('\n');
            }
            text.push_str("PositionGrid\n");
            text.push_str("symbol | last_update | decl_per_share | weeks\n");
            for group in &grid.table2 {
                text.push_str(&format!("group {}\n", group.cadence));
                for row in &group.rows {
                    let mut line = format!(
                        "{} | {} | {}",
                        row.symbol,
                        row.last_update.clone().unwrap_or_default(),
                        fmt_per_share(
                            row.declaration_per_share_minor,
                            row.declaration_per_share_scale
                        )
                    );
                    for cell in &row.cells {
                        line.push_str(" | ");
                        line.push_str(&fmt_cell(cell.amount_minor));
                    }
                    text.push_str(&line);
                    text.push('\n');
                }
            }
        }
    } else {
        text.push_str("WeekDetail\n");
        text.push_str("symbol | last_update | decl_per_share | Plan $ | Declaration $ | Actual $ | Variance\n");
        if let Some(week) = week {
            for (group_name, rows) in week_positions_by_cadence(week) {
                text.push_str(&format!("group {group_name}\n"));
                for pos in rows {
                    text.push_str(&format!(
                        "{} | {} | {} | {} | {} | {} | {}\n",
                        pos.symbol,
                        pos.last_update.clone().unwrap_or_default(),
                        fmt_per_share(pos.declaration_per_share_minor, pos.declaration_per_share_scale),
                        fmt_cell(pos.plan_known.then_some(pos.planned_minor)),
                        fmt_cell(pos.declaration_known.then_some(pos.declaration_minor)),
                        fmt_cell(pos.actual_known.then_some(pos.actual_minor)),
                        if pos.plan_known && pos.actual_known {
                            fmt_cell(Some(pos.actual_minor - pos.planned_minor))
                        } else {
                            String::new()
                        }
                    ));
                }
            }
        }
    }
    simple_pdf(&text, landscape)
}

fn simple_pdf(text: &str, landscape: bool) -> Vec<u8> {
    let (w, h) = if landscape { (792, 612) } else { (612, 792) };
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let mut tj = String::from("BT /F1 9 Tf 36  ");
    tj.push_str(&(h - 40).to_string());
    tj.push_str(" Td 11 TL ");
    for line in escaped.lines() {
        tj.push('(');
        tj.push_str(line);
        tj.push_str(")' T* ");
    }
    tj.push_str("ET");
    let stream = tj.into_bytes();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(format!("<< /Type /Catalog /Pages 2 0 R >>").into_bytes());
    objects.push(format!("<< /Type /Pages /Kids [3 0 R] /Count 1 >>").into_bytes());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
        )
        .into_bytes(),
    );
    let mut contents = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    contents.extend_from_slice(&stream);
    contents.extend_from_slice(b"\nendstream");
    objects.push(contents);
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0u32];
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len() as u32);
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn xlsx_bytes(
    cover: &IncomePlanCoverBody,
    grid: Option<&IncomePlanGridBody>,
    week: Option<&IncomePlanWeekBody>,
) -> Result<Vec<u8>, String> {
    let mut wb = Workbook::new();
    if cover.pattern == "A" {
        let grid = grid.ok_or("pattern A export needs grid")?;
        write_account_rollup(&mut wb, grid)?;
        write_position_grid(&mut wb, grid)?;
        write_cover_sheet(&mut wb, cover)?;
    } else {
        let week = week.ok_or("pattern B export needs week")?;
        write_week_detail(&mut wb, week)?;
        write_cover_sheet(&mut wb, cover)?;
    }
    wb.save_to_buffer().map_err(|e| e.to_string())
}

fn write_cover_sheet(wb: &mut Workbook, cover: &IncomePlanCoverBody) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name("Cover").map_err(|e| e.to_string())?;
    for (i, line) in cover_lines(cover).iter().enumerate() {
        sheet
            .write_string(i as u32, 0, line)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn write_account_rollup(wb: &mut Workbook, grid: &IncomePlanGridBody) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name("AccountRollup").map_err(|e| e.to_string())?;
    for (c, name) in grid.table1_columns.iter().enumerate() {
        if name.eq_ignore_ascii_case("last_update") {
            return Err("AccountRollup must not have last_update".into());
        }
        sheet
            .write_string(0, c as u16, name)
            .map_err(|e| e.to_string())?;
    }
    for (r, row) in grid.table1.iter().enumerate() {
        sheet
            .write_string((r + 1) as u32, 0, &row.label)
            .map_err(|e| e.to_string())?;
        let year = fmt_opt(row.year_minor, row.year_pct_minor);
        sheet
            .write_string((r + 1) as u32, 1, &year)
            .map_err(|e| e.to_string())?;
        for (c, cell) in row.cells.iter().enumerate() {
            let text = if row.id == "delta_to_plan_pct" {
                cell.tone.clone()
            } else {
                fmt_cell(cell.amount_minor)
            };
            sheet
                .write_string((r + 1) as u32, (c + 2) as u16, &text)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn write_position_grid(wb: &mut Workbook, grid: &IncomePlanGridBody) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name("PositionGrid").map_err(|e| e.to_string())?;
    sheet.write_string(0, 0, "symbol").map_err(|e| e.to_string())?;
    sheet
        .write_string(0, 1, "last_update")
        .map_err(|e| e.to_string())?;
    sheet
        .write_string(0, 2, "decl_per_share")
        .map_err(|e| e.to_string())?;
    let week_ends: Vec<String> = grid.weeks.iter().map(|w| w.end.clone()).collect();
    for (i, end) in week_ends.iter().enumerate() {
        sheet
            .write_string(0, (i + 3) as u16, end)
            .map_err(|e| e.to_string())?;
    }
    let mut r = 1u32;
    for group in &grid.table2 {
        sheet
            .write_string(r, 0, &group.cadence)
            .map_err(|e| e.to_string())?;
        r += 1;
        for row in &group.rows {
            sheet
                .write_string(r, 0, &row.symbol)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(r, 1, row.last_update.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(
                    r,
                    2,
                    &fmt_per_share(
                        row.declaration_per_share_minor,
                        row.declaration_per_share_scale,
                    ),
                )
                .map_err(|e| e.to_string())?;
            for (c, cell) in row.cells.iter().enumerate() {
                sheet
                    .write_string(r, (c + 3) as u16, &fmt_cell(cell.amount_minor))
                    .map_err(|e| e.to_string())?;
            }
            r += 1;
        }
    }
    Ok(())
}

fn write_week_detail(wb: &mut Workbook, week: &IncomePlanWeekBody) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name("WeekDetail").map_err(|e| e.to_string())?;
    for (i, h) in [
        "symbol",
        "last_update",
        "decl_per_share",
        "Plan $",
        "Declaration $",
        "Actual $",
        "Variance",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string(0, i as u16, *h)
            .map_err(|e| e.to_string())?;
    }
    let mut r = 1u32;
    for (group_name, rows) in week_positions_by_cadence(week) {
        sheet
            .write_string(r, 0, group_name)
            .map_err(|e| e.to_string())?;
        r += 1;
        for pos in rows {
            sheet
                .write_string(r, 0, &pos.symbol)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(r, 1, pos.last_update.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(
                    r,
                    2,
                    &fmt_per_share(
                        pos.declaration_per_share_minor,
                        pos.declaration_per_share_scale,
                    ),
                )
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(r, 3, &fmt_cell(pos.plan_known.then_some(pos.planned_minor)))
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(
                    r,
                    4,
                    &fmt_cell(pos.declaration_known.then_some(pos.declaration_minor)),
                )
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(r, 5, &fmt_cell(pos.actual_known.then_some(pos.actual_minor)))
                .map_err(|e| e.to_string())?;
            let var = if pos.plan_known && pos.actual_known {
                fmt_cell(Some(pos.actual_minor - pos.planned_minor))
            } else {
                String::new()
            };
            sheet.write_string(r, 6, &var).map_err(|e| e.to_string())?;
            r += 1;
        }
    }
    Ok(())
}

pub fn html_mock_name() -> &'static str {
    HTML_MOCK
}

pub fn uses_html_mock(src: &str) -> bool {
    src.contains(HTML_MOCK)
}

pub fn year_weeks_for(as_of: chrono::NaiveDate) -> Vec<financial_domain::income_plan::GridWeek> {
    use chrono::Duration;
    use financial_domain::week::week_containing;
    let start = week_containing(chrono::NaiveDate::from_ymd_opt(as_of.year(), 1, 1).unwrap());
    let end = week_containing(chrono::NaiveDate::from_ymd_opt(as_of.year(), 12, 31).unwrap());
    let mut out = Vec::new();
    let mut d = start.end;
    while d <= end.end {
        let week = week_containing(d);
        let kind = if week.end < as_of {
            GridWeekKind::Closed
        } else if week.start <= as_of && as_of <= week.end {
            GridWeekKind::InProgress
        } else {
            GridWeekKind::Future
        };
        out.push(financial_domain::income_plan::GridWeek { week, kind });
        d += Duration::days(7);
    }
    out
}
