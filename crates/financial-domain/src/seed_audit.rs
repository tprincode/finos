//! Production-disb parent identity: template money must match the posted ledger.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedDisbursementIdentity {
    pub idempotency_key: String,
    pub amount_minor: i64,
    pub federal_withholding_minor: i64,
    pub state_withholding_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedDisbursementMismatch {
    pub idempotency_key: String,
    pub field: &'static str,
    pub expected: i64,
    pub actual: Option<i64>,
}

/// Compare every template disbursement parent to the posted `production-disb-*` row.
/// Extra ledger rows are ignored so later owner posts do not hide a seed miss.
pub fn seed_disbursement_mismatches(
    expected: &[SeedDisbursementIdentity],
    actual: &[SeedDisbursementIdentity],
) -> Vec<SeedDisbursementMismatch> {
    let by_key: std::collections::HashMap<&str, &SeedDisbursementIdentity> = actual
        .iter()
        .map(|row| (row.idempotency_key.as_str(), row))
        .collect();
    let mut out = Vec::new();
    for exp in expected {
        let Some(got) = by_key.get(exp.idempotency_key.as_str()) else {
            out.push(SeedDisbursementMismatch {
                idempotency_key: exp.idempotency_key.clone(),
                field: "missing",
                expected: exp.amount_minor,
                actual: None,
            });
            continue;
        };
        if got.amount_minor != exp.amount_minor {
            out.push(SeedDisbursementMismatch {
                idempotency_key: exp.idempotency_key.clone(),
                field: "amount_minor",
                expected: exp.amount_minor,
                actual: Some(got.amount_minor),
            });
        }
        if got.federal_withholding_minor != exp.federal_withholding_minor {
            out.push(SeedDisbursementMismatch {
                idempotency_key: exp.idempotency_key.clone(),
                field: "federal_withholding_minor",
                expected: exp.federal_withholding_minor,
                actual: Some(got.federal_withholding_minor),
            });
        }
        if got.state_withholding_minor != exp.state_withholding_minor {
            out.push(SeedDisbursementMismatch {
                idempotency_key: exp.idempotency_key.clone(),
                field: "state_withholding_minor",
                expected: exp.state_withholding_minor,
                actual: Some(got.state_withholding_minor),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, amount: i64, fed: i64, state: i64) -> SeedDisbursementIdentity {
        SeedDisbursementIdentity {
            idempotency_key: key.into(),
            amount_minor: amount,
            federal_withholding_minor: fed,
            state_withholding_minor: state,
        }
    }

    #[test]
    fn zero_withholding_on_a_template_parent_is_a_mismatch() {
        let expected = [row("production-disb-0-2026-a", 110_000, 12_100, 5_500)];
        let actual = [row("production-disb-0-2026-a", 110_000, 0, 0)];
        let miss = seed_disbursement_mismatches(&expected, &actual);
        assert_eq!(miss.len(), 2, "{miss:?}");
        assert!(miss.iter().any(|m| m.field == "federal_withholding_minor"));
        assert!(miss.iter().any(|m| m.field == "state_withholding_minor"));
    }

    #[test]
    fn later_owner_posts_do_not_hide_a_missing_seed_parent() {
        let expected = [row("production-disb-0-2026-a", 110_000, 12_100, 0)];
        let actual = [row("owner-saturday", 110_000, 12_100, 0)];
        let miss = seed_disbursement_mismatches(&expected, &actual);
        assert_eq!(miss[0].field, "missing");
    }

    #[test]
    fn matching_parents_pass() {
        let expected = [row("production-disb-0-2026-a", 110_000, 12_100, 5_500)];
        let actual = [
            row("production-disb-0-2026-a", 110_000, 12_100, 5_500),
            row("owner-saturday", 80_000, 8_000, 0),
        ];
        assert!(seed_disbursement_mismatches(&expected, &actual).is_empty());
    }
}
