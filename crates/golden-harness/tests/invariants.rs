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
