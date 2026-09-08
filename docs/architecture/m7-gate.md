# M7 gate — Release hardening

V1.1 §12: signed installers, update, accessibility, performance, backup and restore drills. **Exit:** owner acceptance matrix and recovery tests pass.

PostgreSQL is Milestone 8. OIDC is Milestone 9. Auto-update stays parked until an update channel is named.

## Commands

```
cargo test -p golden-harness -- recovery
cargo test -p golden-harness -- installer
cargo test -p golden-harness -- performance
cargo test --workspace
```

Signed Windows package (long compile; uses `CurrentUser\My` thumbprint `CEAAACA78BB136405A1496C2DD39EB0E8975D6E6`):

```
npm run desktop:build
```

Produced 2026-08-19 (do not commit `target/`):

`apps/desktop/src-tauri/target/release/bundle/nsis/finos_0.1.0_x64-setup.exe`

Signature: `CN=finos`, thumbprint `CEAAACA78BB136405A1496C2DD39EB0E8975D6E6`. On this PC (2026-08-19): CurrentUser Root + TrustedPublisher; `Get-AuthenticodeSignature` is `Valid`. Installed current-user to `%LOCALAPPDATA%\finos` (`finos-desktop.exe`).

Do not commit `.pfx` files or passwords.

## Slice 1 must pass

| Filter | What it proves |
|--------|----------------|
| `recovery_drill_restore_returns_posted_dividend_to_pre_mutation_state` | SnapshotCreate → post dividend → SnapshotRestore returns DividendGet to 0; hashes match |

## Slice 2 must pass

| Filter | What it proves |
|--------|----------------|
| `signed_windows_installer_nsis_uses_owner_thumbprint` | NSIS current-user bundle; SHA-256; DigiCert timestamp; owner thumbprint |
| `npm run desktop:build` | NSIS setup exe exists and is signed with that thumbprint |

## Slice 3 must pass

| Filter | What it proves |
|--------|----------------|
| `accessibility_primary_actions_have_accessible_names` | Exit and platform actions expose accessible names |

## Slice 4 must pass

| Filter | What it proves |
|--------|----------------|
| `performance_magi_inventory_completes_under_one_second` | MAGI pack inventory loop finishes in under one second |

## Owner acceptance matrix (Windows)

| Item | Status |
|------|--------|
| Recovery drill: restore undoes a posted fact | Slice 1 |
| Signed NSIS binary (`npm run desktop:build`) | Slice 2 — produced 2026-08-19 |
| Accessibility names on primary actions | Slice 3 |
| MAGI inventory performance budget | Slice 4 |
| Trust `CN=finos` / install on this PC | Done 2026-08-19 — current-user Root + TrustedPublisher; `%LOCALAPPDATA%\finos` |
| Auto-update channel | Parked — needs owner channel |
| macOS restore/handoff | Parked — needs a Mac (M1) |

## Out of these slices

Updater, Postgres, OIDC.
