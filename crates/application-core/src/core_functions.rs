//! Owner-facing core function catalog. Same JSON is compiled in so installed
//! releases do not need the repo tree.

use crate::contracts::CoreFunctionsGetBody;

pub const CORE_FUNCTIONS_JSON: &str =
    include_str!("../../../docs/architecture/core-functions.json");
pub const UI_MODULES_JSON: &str = include_str!("../../../docs/architecture/ui-modules.json");

pub fn core_functions_catalog() -> Result<CoreFunctionsGetBody, String> {
    let mut body: CoreFunctionsGetBody =
        serde_json::from_str(CORE_FUNCTIONS_JSON).map_err(|e| e.to_string())?;
    let packed: serde_json::Value =
        serde_json::from_str(UI_MODULES_JSON).map_err(|e| e.to_string())?;
    body.modules = serde_json::from_value(
        packed
            .get("modules")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| e.to_string())?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_has_decl_per_share() {
        let body = core_functions_catalog().expect("catalog");
        assert!(body
            .items
            .iter()
            .any(|i| i.id == "income-week-decl-per-share"));
        assert!(body.items.iter().any(|i| i.id == "file-restart-graceful"));
        assert!(body.items.iter().all(|i| !i.last_changed.is_empty()));
        let restart = body
            .items
            .iter()
            .find(|i| i.id == "file-restart-graceful")
            .expect("restart");
        assert!(restart.also_verify.iter().any(|n| n == "desktop_menu"));
    }
}
