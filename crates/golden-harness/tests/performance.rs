use std::time::{Duration, Instant};

use golden_harness::{magi_pack_inventory, repo_root};

#[test]
fn performance_magi_inventory_completes_under_one_second() {
    let root = repo_root();
    let start = Instant::now();
    for _ in 0..20 {
        magi_pack_inventory(&root).expect("inventory");
    }
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "MAGI pack inventory loop took {:?}",
        start.elapsed()
    );
}
