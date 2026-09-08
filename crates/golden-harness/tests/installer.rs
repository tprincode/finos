use golden_harness::repo_root;
use serde_json::Value;

const FINOS_CODE_SIGN_THUMBPRINT: &str = "CEAAACA78BB136405A1496C2DD39EB0E8975D6E6";

#[test]
fn signed_windows_installer_nsis_uses_owner_thumbprint() {
    let raw = std::fs::read_to_string(
        repo_root().join("apps/desktop/src-tauri/tauri.conf.json"),
    )
    .expect("tauri.conf.json");
    let conf: Value = serde_json::from_str(&raw).expect("json");
    let targets = conf["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets array");
    assert!(
        targets.iter().any(|t| t.as_str() == Some("nsis")),
        "Windows package target must include nsis"
    );
    let windows = &conf["bundle"]["windows"];
    assert_eq!(
        windows["certificateThumbprint"].as_str(),
        Some(FINOS_CODE_SIGN_THUMBPRINT)
    );
    assert_eq!(windows["digestAlgorithm"].as_str(), Some("sha256"));
    assert_eq!(
        windows["timestampUrl"].as_str(),
        Some("http://timestamp.digicert.com")
    );
    assert_eq!(
        windows["nsis"]["installMode"].as_str(),
        Some("currentUser")
    );
}
