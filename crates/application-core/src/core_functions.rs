//! Owner-facing core function catalog. Same JSON is compiled in so household
//! installs do not need the repo tree.

use crate::contracts::CoreFunctionsGetBody;

pub const CORE_FUNCTIONS_JSON: &str =
    include_str!("../../../docs/architecture/core-functions.json");

pub fn core_functions_catalog() -> Result<CoreFunctionsGetBody, String> {
    serde_json::from_str(CORE_FUNCTIONS_JSON).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_has_decl_per_share() {
        let body = core_functions_catalog().expect("catalog");
        assert!(body.items.iter().any(|i| i.id == "income-week-decl-per-share"));
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
