//! Ubiquitous: storage is minor units. Display divides by 10^scale once. Never multiply for display.
//! Mirrors packages/ui-components formatUsd / formatScaled (group thousands, then fraction digits).

use application_core::task::magi_week_title;
use golden_harness::repo_root;

fn group_int(digits: &str) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::new();
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch as char);
    }
    out
}

/// Rust twin of TS formatUsd for USD cash (scale 2) and other fixed scales.
fn format_usd_cents(minor: i64, scale: u8) -> String {
    let places = scale as usize;
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.unsigned_abs().to_string();
    let digits = format!("{:0>width$}", abs, width = places + 1);
    if places == 0 {
        return format!("{sign}${}", group_int(&digits));
    }
    let i = digits.len() - places;
    format!(
        "{sign}${}.{}",
        group_int(&digits[..i]),
        &digits[i..]
    )
}

#[test]
fn locked_household_fixtures_are_cents() {
    assert_eq!(format_usd_cents(248_580, 2), "$2,485.80"); // yesterday's overage
    assert_eq!(format_usd_cents(8_460_000, 2), "$84,600.00"); // 2-person cliff
    assert_eq!(format_usd_cents(1_805_400, 2), "$18,054.00"); // APTC
    assert_eq!(format_usd_cents(975_000, 2), "$9,750.00"); // HSA
    assert_eq!(format_usd_cents(40_032_10, 2), "$40,032.10"); // IRA EOY (4_003_210 cents)
}

#[test]
fn display_never_multiplies() {
    let stored = 248_580; // cents
    let once = format_usd_cents(stored, 2);
    let twice = format_usd_cents(stored * 100, 2);
    assert_eq!(once, "$2,485.80");
    assert_ne!(twice, once);
    assert_eq!(twice, "$248,580.00"); // failure mode — must not appear as "correct" display
}

#[test]
fn tax_magi_task_screens_keep_cents_needles() {
    let root = repo_root();
    let report = std::fs::read_to_string(
        root.join("apps/desktop/src/features/cash/HouseholdIncomeReport.tsx"),
    )
    .expect("HouseholdIncomeReport.tsx");
    let forecast =
        std::fs::read_to_string(root.join("apps/desktop/src/features/cash/magiForecast.ts"))
            .expect("magiForecast.ts");
    let task_ui = format!(
        "{}\n{}",
        std::fs::read_to_string(
            root.join("apps/desktop/src/features/task-manager/TaskManager.tsx"),
        )
        .expect("TaskManager.tsx"),
        std::fs::read_to_string(
            root.join("apps/desktop/src/features/task-manager/ReminderTable.tsx"),
        )
        .expect("ReminderTable.tsx"),
    );
    let week_ahead =
        std::fs::read_to_string(root.join("apps/desktop/src/features/cash/WeekAhead.tsx"))
            .expect("WeekAhead.tsx");
    let task_core =
        std::fs::read_to_string(root.join("crates/application-core/src/task.rs")).expect("task.rs");

    // Constants are cents. Corrupting them by ×100 must fail.
    assert!(
        forecast.contains("JOINT_TWO_PERSON_CLIFF_MINOR = 8_460_000"),
        "cliff stays $84,600 as cents"
    );
    assert!(
        forecast.contains("APPLICATION_APTC_MINOR = 1_805_400"),
        "APTC stays $18,054 as cents"
    );
    assert!(
        forecast.contains("HSA_CONTRIBUTION_MINOR: number = 975_000"),
        "HSA stays $9,750 as cents"
    );
    assert!(
        forecast.contains("NET_CAPITAL_LOSS_LIMIT_MINOR = 300_000")
            && !forecast.contains("NET_CAPITAL_LOSS_LIMIT_MINOR = 3_000")
            && !forecast.contains("NET_CAPITAL_LOSS_LIMIT_MINOR = 30_000_000"),
        "the 1040 net capital-loss limit stays $3,000 as cents"
    );
    assert!(
        !forecast.contains("JOINT_TWO_PERSON_CLIFF_MINOR = 846_000_000")
            && !forecast.contains("APPLICATION_APTC_MINOR = 180_540_000")
            && !forecast.contains("HSA_CONTRIBUTION_MINOR: number = 97_500_000"),
        "do not store dollars×100 as fake cents"
    );

    // Display goes through formatUsd(minor[, scale]) — never formatUsd(x * 100).
    assert!(
        report.contains("formatUsd(minor, scale)") || report.contains("formatUsd(minor,"),
        "HouseholdIncomeReport formats via formatUsd"
    );
    assert!(
        forecast.contains("formatUsd(") && task_ui.contains("formatUsd("),
        "forecast + Task Manager use formatUsd"
    );
    let ui = format!("{report}\n{forecast}\n{task_ui}");
    // Narrow anti-needle: multiply argument into formatUsd (do not ban every "* 100" in the file).
    assert!(
        !ui.lines().any(|line| {
            let t = line.trim();
            t.contains("formatUsd(")
                && (t.contains("* 100)")
                    || t.contains("*100)")
                    || t.contains("* 100,")
                    || t.contains("*100,"))
        }),
        "formatUsd call sites must not multiply by 100"
    );

    // Hardcoded failure-mode dollar strings must not appear as "correct" copy.
    assert!(
        !forecast.contains("$248,580")
            && !report.contains("$248,580")
            && !task_ui.contains("$248,580")
            && !task_core.contains("$248,580")
            && !forecast.contains("$84,600,000")
            && !report.contains("$84,600,000"),
        "do not hardcode ×100 display dollars"
    );

    // Task title path: overage is cents → $2,485.80 once.
    assert_eq!(
        magi_week_title(248_580),
        "MAGI over cliff by $2,485.80 — resolve or snooze till next plan week."
    );
    assert!(
        task_core.contains("fn magi_week_title") && task_core.contains("usd(overage_minor)"),
        "task title formats overage_minor as cents"
    );

    assert!(
        week_ahead.contains("aria-label=\"Week ahead\"")
            && week_ahead.contains("Snooze till tomorrow"),
        "Week Ahead combines tasks and cash; snooze till tomorrow stays labeled"
    );
}

/// Cart per-share is scale 4. Prefilling Add Lot with `/ 100` turns $32.72 into $3,272.00.
#[test]
fn cart_to_add_lot_uses_cart_price_input_not_div_100() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).expect("App.tsx");
    let cart_price =
        std::fs::read_to_string(root.join("apps/desktop/src/features/shopping-cart/cartPrice.ts"))
            .expect("cartPrice.ts");

    assert!(
        cart_price.contains("CART_UNIT_SCALE = 4"),
        "cart unit prices stay scale 4"
    );
    assert!(
        app.contains("cartPriceInput(prefill.lastMinor, prefill.priceScale"),
        "cart → Add Lot must format via cartPriceInput(scale)"
    );
    assert!(
        !app.lines().any(|line| {
            let t = line.trim();
            t.contains("setAddLotCost")
                && t.contains("prefill.lastMinor")
                && (t.contains("/ 100") || t.contains("/100"))
        }),
        "never setAddLotCost(lastMinor / 100) — that is the $32.72 → $3,272 failure"
    );

    // Twin of cartPriceInput(327_200, 4) and 20 × $32.72 → $654.40 cents.
    assert_eq!(cart_price_input_twin(327_200, 4), "32.7200");
    assert_eq!(format_usd_cents(20 * 3_272, 2), "$654.40");
}

/// Avg 3 / Avg 6 stay scale-5 minors until formatPerShare divides once.
#[test]
fn calculator_avg_fields_use_scale_five_not_cents() {
    let root = repo_root();
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx"))
        .expect("ui-components");
    let calc = std::fs::read_to_string(root.join("crates/financial-domain/src/calculator.rs"))
        .expect("calculator.rs");
    assert!(
        calc.contains("PAY_AVG_SCALE: u8 = 5")
            && calc.contains("mean_newest_complete_pays"),
        "domain mean lands at scale 5"
    );
    assert!(
        ui.contains("export function avg3Display")
            && ui.contains("export function avg6Display")
            && ui.contains("formatPerShare(row.avg3Minor")
            && ui.contains("formatPerShare(row.avg6Minor"),
        "UI displays DeclarationHistory avg fields via formatPerShare"
    );
    let sheet = ui
        .split("export function CalculatorReturnSheet")
        .nth(1)
        .unwrap()
        .split("function MonthPerThousandChart")
        .next()
        .unwrap();
    assert!(
        !sheet.contains("formatUsd(avg3")
            && !sheet.contains("avg6Label(")
            && !sheet.contains("meanNewestPays("),
        "Calculator sheet must not recompute or cents-format Avg 3/6"
    );
}

/// Calculator Excel keeps Plan/Price as integer minor + scale until write_number_with_format.
#[test]
fn calculator_export_keeps_minor_scale_until_excel_write() {
    let root = repo_root();
    let core = std::fs::read_to_string(root.join("crates/application-core/src/component_export.rs"))
        .expect("component_export.rs");
    let typed = core
        .split("async fn calculator_typed_sheet(")
        .nth(1)
        .expect("calculator_typed_sheet")
        .split("async fn trends_sheet(")
        .next()
        .expect("trends_sheet after typed calculator");
    assert!(
        typed.contains("money_cell(")
            && typed.contains("Cell::Money {")
            && typed.contains("Cell::Percent {")
            && typed.contains("h.avg3_minor")
            && typed.contains("h.avg6_minor")
            && !typed.contains("mean_newest_pays")
            && !typed.contains("money_text("),
        "typed Calculator rows carry minor/scale; Avg 3/6 come from DeclarationHistory"
    );
    assert!(
        core.contains("let major = *minor as f64 / 10f64.powi(i32::from(*scale))")
            && core.contains("write_number_with_format"),
        "Excel write divides minor by 10^scale once"
    );
}

fn cart_price_input_twin(minor: i64, scale: u8) -> String {
    let s = scale as usize;
    let abs = minor.unsigned_abs().to_string();
    let digits = format!("{:0>width$}", abs, width = s + 1);
    let i = digits.len() - s;
    let sign = if minor < 0 { "-" } else { "" };
    if s == 0 {
        format!("{sign}{}", &digits)
    } else {
        format!("{sign}{}.{}", &digits[..i], &digits[i..])
    }
}
