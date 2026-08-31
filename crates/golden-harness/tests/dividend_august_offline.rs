//! Offline: Finos broker parse of the owner Fidelity August CSV vs ROI update xlsx.
//! Does not post. Skips Account 9. Skips entirely if the owner files are absent.

use import_engine::{
    dividend_gap, find_broker_header, is_account_9, parse_broker_csv_detail,
    parse_roi_dividend_update, BrokerLayout, DividendFact, GapKind,
};
use std::path::Path;

const FIDELITY_CSV: &str =
    r"c:\Users\EVTom\Documents\Financial\Fidelity Transactions for August2026.csv";
const ROI_XLSX: &str = r"c:\Users\EVTom\Documents\Financial\ROI transactions update.xlsx";

#[test]
#[ignore = "owner August Fidelity + ROI files; run with --ignored"]
fn august_fidelity_import_matches_roi_update_excluding_account_9() {
    let csv_path = Path::new(FIDELITY_CSV);
    let xlsx_path = Path::new(ROI_XLSX);
    if !csv_path.is_file() || !xlsx_path.is_file() {
        eprintln!("owner August files not present; skip");
        return;
    }
    let csv = std::fs::read_to_string(csv_path).unwrap();
    let (layout, _) = find_broker_header(&csv).expect("Fidelity activity header");
    assert_eq!(layout, BrokerLayout::Fidelity);
    let parsed = parse_broker_csv_detail(layout, &csv, None).unwrap();
    let import: Vec<DividendFact> = parsed
        .candidates
        .iter()
        .filter(|c| !is_account_9(&c.account_name))
        .filter_map(DividendFact::from_candidate)
        .collect();
    let roi = parse_roi_dividend_update(xlsx_path).expect("ROI xlsx");
    let report = dividend_gap(&import, &roi, &[]);
    let matched = report.iter().filter(|r| r.kind == GapKind::Match).count();
    let only_import: Vec<_> = report
        .iter()
        .filter(|r| r.kind == GapKind::OnlyBroker)
        .map(|r| format!("{} {} {} {}", r.fact.account_name, r.fact.symbol, r.fact.occurred_on, r.fact.amount_minor))
        .collect();
    let only_roi: Vec<_> = report
        .iter()
        .filter(|r| r.kind == GapKind::OnlySheet)
        .map(|r| format!("{} {} {} {}", r.fact.account_name, r.fact.symbol, r.fact.occurred_on, r.fact.amount_minor))
        .collect();
    eprintln!(
        "import {} roi {} matched {} only_import {} only_roi {} dropped {}",
        import.len(),
        roi.len(),
        matched,
        only_import.len(),
        only_roi.len(),
        parsed.dropped.len()
    );
    for row in &only_import {
        eprintln!("ONLY IMPORT {row}");
    }
    for row in &only_roi {
        eprintln!("ONLY ROI {row}");
    }
    assert!(
        only_import.is_empty() && only_roi.is_empty(),
        "Finos import of the Fidelity CSV does not reproduce the ROI update (excl Account 9). matched={matched} only_import={} only_roi={}",
        only_import.len(),
        only_roi.len()
    );
}
