use golden_harness::{
    load_production_expected, load_production_seed_via_commands, production_seed_actual_counts,
    production_seed_actual_totals, production_template_totals, repo_root,
};
use storage_sqlite::LocalPlatform;

#[tokio::test]
async fn production_seed_counts_reconcile() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    assert!(
        production.join("Template_Accounts.xlsx").exists(),
        "unzip the Include Package so templates land at database/seed/production/"
    );

    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("production seed load via commands");

    let expected = load_production_expected(&production.join("expected-production.yaml"))
        .expect("expected-production.yaml");
    let actual_counts = production_seed_actual_counts(&platform)
        .await
        .expect("actual counts");
    assert_eq!(actual_counts, expected.counts);

    let template_totals =
        production_template_totals(&production).expect("template money from xlsx");
    assert_eq!(
        template_totals, expected.totals,
        "expected-production.yaml money must match the locked templates"
    );

    let actual_totals = production_seed_actual_totals(&platform)
        .await
        .expect("actual money");
    assert_eq!(
        actual_totals.yield_amount_minor, expected.totals.yield_amount_minor,
        "DividendGet actual total must match owner-approved yield"
    );
    assert_eq!(
        actual_totals.disbursement_gross_minor, expected.totals.disbursement_gross_minor,
        "posted disbursement gross must match owner-approved total"
    );

    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("second load is a no-op");
    let again = production_seed_actual_counts(&platform)
        .await
        .expect("counts after second load");
    assert_eq!(again, expected.counts);
}
