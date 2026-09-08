use application_core::ports::canonical::Canonical;
use golden_harness::{load_expected, load_seed_via_commands, repo_root};
use storage_sqlite::LocalPlatform;

#[tokio::test]
async fn seed_counts_and_totals_reconcile() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    load_seed_via_commands(&platform, &root.join("database/seed/fixture.yaml"))
        .await
        .expect("seed load via commands");
    let actual = platform.reconcile_counts().await.expect("counts");
    let expected = load_expected(&root.join("database/seed/expected.yaml")).expect("expected");
    assert_eq!(golden_harness::ExpectedCounts::from(actual), expected);
}

#[tokio::test]
async fn canonical_week_is_saturday_to_friday() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let week = platform
        .canonical_week_get("2026-08-18".into())
        .await
        .unwrap();
    assert_eq!(week.start, "2026-08-15");
    assert_eq!(week.end, "2026-08-21");
    assert_eq!(week.week_year, 2026);
    assert_eq!(week.week_number, 33);
}
