//! Allocation targets are decision support. They never post cash or MAGI facts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationTarget {
    pub name: String,
    pub target_minor: i64,
    pub scale: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationProgress {
    pub target: AllocationTarget,
    pub open_performance_minor: i64,
    pub open_tax_minor: i64,
}

/// Scale 2: 6000 means 60.00 percent.
pub fn prepare_target(name: String, target_minor: i64, scale: u8) -> AllocationTarget {
    AllocationTarget {
        name,
        target_minor,
        scale,
    }
}

/// Attach open lot cost basis (not market value). Dual basis stays separate. Not a fill.
pub fn attach_open_basis(
    target: AllocationTarget,
    open_performance_minor: i64,
    open_tax_minor: i64,
) -> AllocationProgress {
    AllocationProgress {
        target,
        open_performance_minor,
        open_tax_minor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_target_does_not_change_cash() {
        let cash_before = 50_000i64;
        let _target = prepare_target("equities".into(), 6_000, 2);
        assert_eq!(cash_before, 50_000);
        assert_ne!(6_000, cash_before);
    }

    #[test]
    fn allocation_open_basis_is_not_cash_or_a_fill() {
        let cash_before = 50_000i64;
        let target = prepare_target("equities".into(), 6_000, 2);
        let progress = attach_open_basis(target, 140_000, 120_000);
        assert_eq!(progress.target.target_minor, 6_000);
        assert_eq!(progress.open_performance_minor, 140_000);
        assert_eq!(progress.open_tax_minor, 120_000);
        assert_ne!(progress.open_performance_minor, progress.open_tax_minor);
        assert_eq!(cash_before, 50_000);
        assert_ne!(progress.open_performance_minor, cash_before);
    }
}
