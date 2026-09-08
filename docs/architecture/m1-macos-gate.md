# M1 macOS gate

Windows M1 remainder can merge after `cargo test` is green on Windows. Milestone 1 is not fully closed until the same restore and handoff cases run on a Mac.

Do not place the live SQLite file in iCloud Drive (or any synced folder). Use the OS local app-data directory.

## Commands

From the repo root:

```bash
cargo test
cd apps/desktop && npm install && npm run tauri dev
```

Confirm the desktop window shows `HealthGet` and `HandoffStatusGet` side by side. Ordinary writes (`ConfigSet` / Save device name) must stay disabled while handoff status is blocked.

## Cases to run on macOS

These match V1.1 §7.1 and the Windows library tests in `application-core` (comparison matrix) and `storage-sqlite` (WAL + restore).

1. **WAL + migrate** — temp DB opens in WAL mode; first launch creates persistent `device_id` / `device_name`.
2. **Equal heads** — create a snapshot (publishes to the local catalog). App opens normally; writes allowed.
3. **Published newer** — catalog published pointer is a descendant of local head. Writes blocked until `SnapshotRestore` or `HandoffResolve` action `review`.
4. **Local newer** — local head descends from published; writes allowed; reminder to publish after checkpoint.
5. **Branch conflict** — neither head descends from the other; both states preserved; no silent merge; ordinary writes stay blocked.
6. **Catalog unavailable** — unverified-handoff; do not claim current; controlled writes allowed.
7. **Hash/validation fail** — reject the candidate; last verified local state retained.
8. **Restore round-trip** — `SnapshotCreate` → mutate local state → `SnapshotRestore` → bundle `database_hash` matches.

## Exit

M1 is closed when this checklist is recorded as passing on macOS with the same git revision that passed on Windows.
