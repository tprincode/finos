//! apps/web on RemoteHttpFinanceClient (AC-ARCH-01). No UI SQL.

use golden_harness::{desktop_ui_contains_no_sql, repo_root};

#[test]
fn web_client_compiles_and_contains_no_sql() {
    let root = repo_root();
    let src = root.join("apps/web/src");
    desktop_ui_contains_no_sql(&src).expect("web UI SQL invariant");

    let app = std::fs::read_to_string(src.join("App.tsx")).unwrap();
    assert!(
        app.contains("RemoteHttpFinanceClient"),
        "apps/web must use RemoteHttpFinanceClient"
    );
    assert!(
        !app.contains("LocalTauriFinanceClient"),
        "apps/web must not use LocalTauriFinanceClient"
    );

    let client = std::fs::read_to_string(src.join("financeClient.ts")).unwrap();
    assert!(
        client.contains("export class RemoteHttpFinanceClient"),
        "web RemoteHttpFinanceClient must exist"
    );
    assert!(
        client.contains("authorization"),
        "web client must send Bearer JWT"
    );

    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let install = std::process::Command::new(npm)
        .arg("install")
        .current_dir(&root)
        .status()
        .unwrap_or_else(|e| panic!("{npm} missing: {e}"));
    assert!(install.success(), "{npm} install failed");
    let status = std::process::Command::new(npm)
        .args(["run", "build", "--workspace=@finos/web"])
        .current_dir(&root)
        .status()
        .unwrap_or_else(|e| panic!("{npm} missing: {e}"));
    assert!(status.success(), "{npm} run build --workspace=@finos/web failed");
}
