//! Checking for a signed app update is not a posted financial activity.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdaterCheck {
    pub applied: bool,
    pub posted: bool,
    pub status: &'static str,
}

/// Fail closed: never apply an update and never post ledger facts.
pub fn check_for_update() -> UpdaterCheck {
    UpdaterCheck {
        applied: false,
        posted: false,
        status: "fail-closed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_check_is_not_a_posted_activity() {
        let posted_before = 50_000i64;
        let check = check_for_update();
        assert!(!check.applied);
        assert!(!check.posted);
        assert_eq!(check.status, "fail-closed");
        assert_eq!(posted_before, 50_000);
    }
}
