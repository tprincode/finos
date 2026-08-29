//! Live DIV-1 adapter audit — runs every production symbol collector and prints a table.
//!
//! Usage:
//!   cargo run -p golden-harness --bin div1-adapter-audit
//!   cargo run -p golden-harness --bin div1-adapter-audit -- NVDW YBTC

use financial_domain::declaration_lookback::{
    validate_paid_lookback, LookbackValidation, DECLARATION_LOOKBACK_TARGET,
};
use financial_domain::div1::{declaration_source_for_provider, is_div1};
use import_engine::{collect_declarations_for, parse_production_templates, DeclarationTarget};
use std::env;
use std::path::PathBuf;

struct Row {
    symbol: String,
    provider: String,
    adapter: String,
    frequency: String,
    paid_qty: usize,
    upcoming_qty: usize,
    required: u8,
    current_date: String,
    current_amount: String,
    status: String,
    detail: String,
}

fn main() {
    let filter: Vec<String> = env::args().skip(1).map(|s| s.to_ascii_uppercase()).collect();
    let production = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/seed/production");
    let doc = parse_production_templates(&production).unwrap_or_else(|e| {
        eprintln!("failed to parse production templates: {e}");
        std::process::exit(1);
    });

    let mut rows = Vec::new();
    let as_of = chrono::Utc::now().date_naive().to_string();

    let mut div1: Vec<_> = doc
        .characteristics
        .iter()
        .filter(|c| is_div1(&c.div_type))
        .collect();
    div1.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    for ch in div1 {
        if !filter.is_empty() && !filter.iter().any(|f| f == &ch.symbol.to_ascii_uppercase()) {
            continue;
        }
        let adapter = declaration_source_for_provider(&ch.provider).unwrap_or("unassigned");
        eprint!("Collecting {} via {}… ", ch.symbol, adapter);
        let target = DeclarationTarget {
            security_id: format!("audit-{}", ch.symbol),
            symbol: ch.symbol.clone(),
            declaration_source: adapter.to_string(),
            source_symbol: ch.symbol.clone(),
            source_url: String::new(),
            force_refresh: true,
            payment_frequency: ch.payment_frequency.clone(),
            div_type: ch.div_type.clone(),
            ..Default::default()
        };
        let outcome = collect_declarations_for(vec![target]);
        eprintln!("done");

        let paid: Vec<_> = outcome
            .candidates
            .iter()
            .filter(|c| {
                c.get("amountPerShareMinor")
                    .and_then(|v| v.as_i64())
                    .map(|a| a > 0)
                    .unwrap_or(false)
            })
            .collect();
        let paid_qty = paid.len();
        let upcoming_qty = outcome.pay_dates.len();

        let latest = paid.iter().max_by(|a, b| {
            a.get("paymentPeriod")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("paymentPeriod").and_then(|v| v.as_str()).unwrap_or(""))
        });
        let (current_date, current_amount) = if let Some(c) = latest {
            let date = c
                .get("paymentPeriod")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let minor = c.get("amountPerShareMinor").and_then(|v| v.as_i64()).unwrap_or(0);
            let scale = c.get("amountScale").and_then(|v| v.as_u64()).unwrap_or(4) as u32;
            let whole = minor / 10i64.pow(scale);
            let frac = (minor % 10i64.pow(scale)).abs();
            let frac_str = format!("{:0width$}", frac, width = scale as usize);
            (date, format!("{whole}.{frac_str}"))
        } else {
            (String::new(), String::from("—"))
        };

        let gate = validate_paid_lookback(
            paid_qty as u8,
            "",
            &as_of,
            &ch.payment_frequency,
        );
        let (required, status, detail) = match &gate {
            LookbackValidation::Complete => (
                DECLARATION_LOOKBACK_TARGET,
                "OK".to_string(),
                String::new(),
            ),
            LookbackValidation::CompleteViaInception { expected, .. } => (
                *expected,
                "OK".to_string(),
                "inception-short".into(),
            ),
            LookbackValidation::ShortWithoutInception { paid } => (
                DECLARATION_LOOKBACK_TARGET,
                "MISS".to_string(),
                format!("{paid}/{DECLARATION_LOOKBACK_TARGET} paid"),
            ),
            LookbackValidation::ShortWithInception { paid, expected } => (
                *expected,
                "MISS".to_string(),
                format!("{paid}/{expected} paid (inception)"),
            ),
        };

        if !outcome.misses.is_empty() {
            let code = outcome.misses[0]
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("miss");
            let reason = outcome.misses[0]
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            rows.push(Row {
                symbol: ch.symbol.clone(),
                provider: ch.provider.clone(),
                adapter: adapter.to_string(),
                frequency: ch.payment_frequency.clone(),
                paid_qty,
                upcoming_qty,
                required,
                current_date,
                current_amount,
                status: "MISS".into(),
                detail: if reason.len() > 60 {
                    format!("{code}: {}…", &reason[..57])
                } else {
                    format!("{code}: {reason}")
                },
            });
            continue;
        }

        rows.push(Row {
            symbol: ch.symbol.clone(),
            provider: ch.provider.clone(),
            adapter: adapter.to_string(),
            frequency: ch.payment_frequency.clone(),
            paid_qty,
            upcoming_qty,
            required,
            current_date,
            current_amount,
            status,
            detail,
        });
    }

    println!();
    println!(
        "DIV-1 adapter audit  as_of={as_of}  symbols={}",
        rows.len()
    );
    println!();
    println!(
        "{:<6} {:<12} {:<14} {:<8} {:>5} {:>5} {:>4} {:<12} {:<10} {:<4} {}",
        "Symbol",
        "Adapter",
        "Provider",
        "Cadence",
        "Paid",
        "Fut",
        "Req",
        "CurDate",
        "CurAmt",
        "Stat",
        "Detail"
    );
    println!("{}", "-".repeat(120));
    let mut miss_count = 0usize;
    for r in &rows {
        if r.status != "OK" {
            miss_count += 1;
        }
        println!(
            "{:<6} {:<12} {:<14} {:<8} {:>5} {:>5} {:>4} {:<12} {:<10} {:<4} {}",
            r.symbol,
            r.adapter,
            truncate(&r.provider, 14),
            truncate(&r.frequency, 8),
            r.paid_qty,
            r.upcoming_qty,
            r.required,
            truncate(&r.current_date, 12),
            truncate(&r.current_amount, 10),
            r.status,
            truncate(&r.detail, 40),
        );
    }
    println!();
    println!(
        "Summary: {} OK, {} MISS (of {} symbols)",
        rows.len() - miss_count,
        miss_count,
        rows.len()
    );

    if miss_count > 0 {
        std::process::exit(1);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}
