//! Immutable snapshot lineage and controlled device handoff (ARCH-05, ADR-0007).

mod catalog;
mod hash;

pub use application_core::contracts::SnapshotIdentity as SnapshotManifest;
pub use catalog::{CatalogError, PublishedHead, SnapshotCatalog};
pub use hash::sha256_file;
