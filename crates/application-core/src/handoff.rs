//! V1.1 §7.1 startup comparison: local head vs published head (pure, no I/O).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::contracts::{HandoffDecision, SnapshotIdentity};

/// How the published catalog presented at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedView<'a> {
    /// Catalog directory missing or unreadable; do not claim current.
    Unavailable,
    /// Manifest/hash/validation failed; keep last verified local state.
    Invalid,
    /// Catalog exists but has no published pointer.
    Missing,
    Head(&'a SnapshotIdentity),
}

/// True when `descendant` can walk `parent_of` links to `ancestor` (inclusive).
pub fn descends_from(
    descendant: Uuid,
    ancestor: Uuid,
    parent_of: &HashMap<Uuid, Option<Uuid>>,
) -> bool {
    if descendant == ancestor {
        return true;
    }
    let mut current = descendant;
    let mut seen = HashSet::new();
    for _ in 0..1024 {
        if !seen.insert(current) {
            return false;
        }
        match parent_of.get(&current) {
            Some(Some(parent)) => {
                if *parent == ancestor {
                    return true;
                }
                current = *parent;
            }
            _ => return false,
        }
    }
    false
}

/// Compare local vs published heads (V1.1 §7.1). `parent_of` maps snapshot_id → parent.
pub fn compare_heads(
    local: Option<&SnapshotIdentity>,
    published: PublishedView<'_>,
    parent_of: &HashMap<Uuid, Option<Uuid>>,
) -> HandoffDecision {
    match published {
        PublishedView::Unavailable => HandoffDecision::UnverifiedHandoff,
        PublishedView::Invalid => HandoffDecision::RejectInvalidKeepLast,
        PublishedView::Missing => {
            if local.is_some() {
                HandoffDecision::AllowWritePublishPending
            } else {
                HandoffDecision::OpenNormally
            }
        }
        PublishedView::Head(published_head) => match local {
            None => HandoffDecision::BlockUntilRestore,
            Some(local_head) if local_head.snapshot_id == published_head.snapshot_id => {
                HandoffDecision::OpenNormally
            }
            Some(local_head) => {
                let published_from_local = descends_from(
                    published_head.snapshot_id,
                    local_head.snapshot_id,
                    parent_of,
                );
                let local_from_published = descends_from(
                    local_head.snapshot_id,
                    published_head.snapshot_id,
                    parent_of,
                );
                if published_from_local {
                    HandoffDecision::BlockUntilRestore
                } else if local_from_published {
                    HandoffDecision::AllowWritePublishPending
                } else {
                    HandoffDecision::BranchConflict
                }
            }
        },
    }
}

/// Ordinary writes: blocked until restore or explicit review for a newer published head;
/// branch conflict never allows silent merge; invalid published keeps local verified writes.
pub fn writes_allowed(decision: HandoffDecision, review_acknowledged: bool) -> bool {
    match decision {
        HandoffDecision::OpenNormally
        | HandoffDecision::AllowWritePublishPending
        | HandoffDecision::UnverifiedHandoff
        | HandoffDecision::RejectInvalidKeepLast => true,
        HandoffDecision::BlockUntilRestore => review_acknowledged,
        HandoffDecision::BranchConflict => false,
    }
}

pub fn decision_message(decision: HandoffDecision) -> &'static str {
    match decision {
        HandoffDecision::OpenNormally => "Local head equals published head; open normally.",
        HandoffDecision::BlockUntilRestore => {
            "Published head is newer; block ordinary writes until restore or explicit review."
        }
        HandoffDecision::AllowWritePublishPending => {
            "Local head is newer; write allowed. Publish a verified snapshot after the next checkpoint."
        }
        HandoffDecision::BranchConflict => {
            "Neither head descends from the other; preserve both states. No silent merge."
        }
        HandoffDecision::UnverifiedHandoff => {
            "Snapshot catalog unavailable; controlled offline work. This computer is not claimed current."
        }
        HandoffDecision::RejectInvalidKeepLast => {
            "Published snapshot failed hash or validation; last verified local state retained."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{CALCULATION_VERSION, SCHEMA_VERSION};

    fn head(id: Uuid, parent: Option<Uuid>) -> SnapshotIdentity {
        SnapshotIdentity {
            database_id: Uuid::nil(),
            snapshot_id: id,
            parent_snapshot_id: parent,
            device_id: Uuid::nil(),
            device_name: "test".to_string(),
            change_sequence: 1,
            last_event_at: "2026-01-01T00:00:00Z".to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            calculation_version: CALCULATION_VERSION.to_string(),
            app_version: "0.1.0".to_string(),
            database_hash: "abc".to_string(),
            evidence_manifest_hash: "def".to_string(),
            validation_status: "valid".to_string(),
            restore_test_status: "not_run".to_string(),
        }
    }

    fn parents(pairs: &[(Uuid, Option<Uuid>)]) -> HashMap<Uuid, Option<Uuid>> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn equal_heads_open_normally() {
        let a = Uuid::from_u128(1);
        let local = head(a, None);
        let map = parents(&[(a, None)]);
        assert_eq!(
            compare_heads(Some(&local), PublishedView::Head(&local), &map),
            HandoffDecision::OpenNormally
        );
        assert!(writes_allowed(HandoffDecision::OpenNormally, false));
    }

    #[test]
    fn published_descends_from_local_blocks_writes() {
        let local_id = Uuid::from_u128(1);
        let published_id = Uuid::from_u128(2);
        let local = head(local_id, None);
        let published = head(published_id, Some(local_id));
        let map = parents(&[(local_id, None), (published_id, Some(local_id))]);
        let decision = compare_heads(Some(&local), PublishedView::Head(&published), &map);
        assert_eq!(decision, HandoffDecision::BlockUntilRestore);
        assert!(!writes_allowed(decision, false));
        assert!(writes_allowed(decision, true));
    }

    #[test]
    fn local_descends_from_published_allows_write() {
        let published_id = Uuid::from_u128(1);
        let local_id = Uuid::from_u128(2);
        let published = head(published_id, None);
        let local = head(local_id, Some(published_id));
        let map = parents(&[(published_id, None), (local_id, Some(published_id))]);
        let decision = compare_heads(Some(&local), PublishedView::Head(&published), &map);
        assert_eq!(decision, HandoffDecision::AllowWritePublishPending);
        assert!(writes_allowed(decision, false));
    }

    #[test]
    fn neither_descends_is_branch_conflict() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let local = head(a, None);
        let published = head(b, None);
        let map = parents(&[(a, None), (b, None)]);
        let decision = compare_heads(Some(&local), PublishedView::Head(&published), &map);
        assert_eq!(decision, HandoffDecision::BranchConflict);
        assert!(!writes_allowed(decision, true));
    }

    #[test]
    fn catalog_unavailable_is_unverified_handoff() {
        let local = head(Uuid::from_u128(1), None);
        let decision = compare_heads(Some(&local), PublishedView::Unavailable, &HashMap::new());
        assert_eq!(decision, HandoffDecision::UnverifiedHandoff);
        assert!(writes_allowed(decision, false));
    }

    #[test]
    fn hash_validation_failure_rejects_candidate() {
        let local = head(Uuid::from_u128(1), None);
        let decision = compare_heads(Some(&local), PublishedView::Invalid, &HashMap::new());
        assert_eq!(decision, HandoffDecision::RejectInvalidKeepLast);
        assert!(writes_allowed(decision, false));
    }

    #[test]
    fn longer_chain_still_detects_descent() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        let local = head(a, None);
        let published = head(c, Some(b));
        let map = parents(&[(a, None), (b, Some(a)), (c, Some(b))]);
        assert_eq!(
            compare_heads(Some(&local), PublishedView::Head(&published), &map),
            HandoffDecision::BlockUntilRestore
        );
    }
}
