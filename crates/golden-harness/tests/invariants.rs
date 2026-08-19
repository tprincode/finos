use golden_harness::{core_crate_tomls_forbid_sqlx_and_tauri, desktop_ui_contains_no_sql, repo_root};

#[test]
fn application_core_and_financial_domain_do_not_depend_on_sqlx_or_tauri() {
    core_crate_tomls_forbid_sqlx_and_tauri(&repo_root()).expect("crate graph invariant");
}

#[test]
fn desktop_react_contains_no_sql() {
    let src = repo_root().join("apps/desktop/src");
    desktop_ui_contains_no_sql(&src).expect("UI SQL invariant");
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
