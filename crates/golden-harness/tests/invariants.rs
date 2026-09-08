use golden_harness::{core_crate_tomls_forbid_sqlx_and_tauri, desktop_ui_contains_no_sql, repo_root};

#[test]
fn application_core_and_financial_domain_do_not_depend_on_sqlx_or_tauri() {
    core_crate_tomls_forbid_sqlx_and_tauri(&repo_root()).expect("crate graph invariant");
}

#[test]
fn desktop_react_contains_no_sql() {
    let root = repo_root();
    desktop_ui_contains_no_sql(&root.join("apps/desktop/src")).expect("UI SQL invariant");
    desktop_ui_contains_no_sql(&root.join("packages/ui-components/src"))
        .expect("ui-components SQL invariant");
}

#[test]
fn web_react_contains_no_sql() {
    let src = repo_root().join("apps/web/src");
    desktop_ui_contains_no_sql(&src).expect("web UI SQL invariant");
}

#[test]
fn desktop_stays_on_local_tauri_and_exposes_remote_http_client() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("LocalTauriFinanceClient"),
        "App.tsx must stay on LocalTauriFinanceClient (no authority cutover)"
    );
    assert!(
        !app.contains("RemoteHttpFinanceClient"),
        "App.tsx must not switch to RemoteHttpFinanceClient"
    );
    let client = std::fs::read_to_string(root.join("apps/desktop/src/financeClient.ts")).unwrap();
    assert!(
        client.contains("export class RemoteHttpFinanceClient"),
        "RemoteHttpFinanceClient must exist for the HTTP path"
    );
    assert!(
        !client.to_ascii_lowercase().contains("select "),
        "FinanceClient must not embed SQL"
    );
}

#[test]
fn financial_pr_workflow_runs_magi_gate_and_does_not_write_oracles() {
    let wf = std::fs::read_to_string(
        repo_root().join(".github/workflows/financial-pr.yml"),
    )
    .expect("financial-pr workflow");
    assert!(
        wf.contains("cargo test -p golden-harness --features magi-gate"),
        "financial PRs must run magi-gate"
    );
    assert!(
        !wf.contains("--ignored"),
        "magi-gate must not be ignored"
    );
    assert!(
        !wf.contains("continue-on-error"),
        "magi-gate must not continue on error"
    );
    assert!(
        wf.contains("Never rewrite owner-approved MAGI oracles"),
        "workflow must forbid MAGI oracle rewrites"
    );
    assert!(
        wf.contains("seed_counts"),
        "workflow must run seed count tests"
    );
}
