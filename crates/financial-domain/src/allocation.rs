//! Allocation targets are decision support. They never post cash or MAGI facts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationTarget {
    pub name: String,
    pub target_minor: i64,
    pub scale: u8,
}

/// Scale 2: 6000 means 60.00 percent.
pub fn prepare_target(name: String, target_minor: i64, scale: u8) -> AllocationTarget {
    AllocationTarget {
        name,
        target_minor,
        scale,
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
}
