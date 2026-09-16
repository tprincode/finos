# Restart Application — owner gate (installed release)

## Rule

Agents and CI must not mark File → Restart Application as fixed, done, or verified for the **installed release** path unless the owner has completed the Windows installed-build repro and this tree records attestation.

Linux CI green, coding-path supervisor relaunch, string sentinels, and PR merge are **not** that evidence.

## Two paths (do not conflate)

| Path | How Restart works | What proves it |
|------|-------------------|----------------|
| Coding | `restart.token` + `start-finos-supervisor.bat` → `start-finos-dev.bat` | Windows coding stack with supervisor |
| Installed release | release `app_restart` → `close_for_shutdown` → `app.restart()` **without** destroy-first | Owner on `%LOCALAPPDATA%\finos\finos-desktop.exe` after NSIS install |

## Static anti-regression (not owner proof)

`installed_release_restart_must_not_destroy_windows_before_app_restart` in `crates/golden-harness/tests/desktop_menu.rs` must stay green. It only proves the destroy-before-`app.restart()` pattern is absent from `lib.rs` text.

## Owner proof (required for Done)

See Project audit: store `docs/restart-false-claim-audit.md` §5. Minimum:

1. Quit all finos processes.
2. `npm run desktop:build` + run NSIS setup.
3. Launch only `%LOCALAPPDATA%\finos\finos-desktop.exe`.
4. File → Restart Application.
5. Pass: new window/process without manual relaunch.

Then set `docs/architecture/restart-owner-attestation.md` fields and set `file-restart-graceful.lastVerified` to the attestation date.

Until then, `lastVerified` **must** remain `UNVERIFIED-owner-windows-installed`.

## Script

```bash
bash scripts/restart-owner-gate.sh
```

Fails if:

- destroy-before-restart static test fails, or
- catalog `lastVerified` is a calendar date without `OWNER_CONFIRMED_INSTALLED_RESTART: true` in the attestation file.
