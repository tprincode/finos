use golden_harness::{
    compare_magi_pack, magi_pack_run, magi_pack_verdict, write_evidence_report, EvidenceReport,
    MagiPackVerdict, repo_root,
};

#[test]
fn approved_magi_oracles_exist_with_locked_rule() {
    let inv = golden_harness::magi_pack_inventory(&repo_root()).expect("inventory");
    let expected: Vec<String> = (1..=10).map(|n| format!("G-MAGI-{n:02}")).collect();
    assert_eq!(inv.scenario_ids, expected);
    assert_eq!(inv.oracle_ids, expected);
    assert!(inv.facts_present);
    assert!(inv.all_scenarios_approved);
    assert!(inv.all_oracles_approved);
    assert!(!inv.all_pending_owner);
    assert!(!inv.all_oracles_proposed);
    assert_eq!(inv.threshold_minor, 8_460_000);
    assert_eq!(inv.safety_reserve_minor, 500_000);
}

#[test]
fn approval_alone_does_not_pass_the_sync_verdict() {
    let dir = repo_root().join("tests/golden/scenarios");
    let verdict = magi_pack_verdict(&dir);
    assert_ne!(
        verdict,
        MagiPackVerdict::Passed,
        "owner approval without a production-path compare must not report Passed"
    );
    assert_eq!(verdict, MagiPackVerdict::Failed);
}

#[test]
fn evidence_report_is_written_without_approving_oracles() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_evidence_report(
        dir.path(),
        &EvidenceReport {
            scenario_id: "G-MAGI-01".into(),
            scenario_version: "0.1.0-stub".into(),
            rule_set_id: "pending".into(),
            calculation_version: "0".into(),
            git_commit: None,
            adapter: "sqlite".into(),
            platform: "windows".into(),
            recorded_at: "2026-08-18T00:00:00Z".into(),
            verdict: "blocked-pending-owner".into(),
            diffs: vec!["oracle pending-owner".into()],
        },
    )
    .unwrap();
    let raw = std::fs::read_to_string(path).unwrap();
    assert!(raw.contains("blocked-pending-owner"));
    assert!(!raw.contains("\"passed\": true"));
}

#[tokio::test]
#[cfg_attr(
    not(feature = "magi-gate"),
    ignore = "M5 gate; run with cargo test -p golden-harness --features magi-gate"
)]
async fn m5_magi_pack_must_pass() {
    let root = repo_root();
    compare_magi_pack(&root)
        .await
        .expect("production-path MAGI compare");
    assert_eq!(
        magi_pack_run(&root).await,
        MagiPackVerdict::Passed,
        "M5 requires owner-approved MAGI oracles and a passing production-path compare"
    );
}
