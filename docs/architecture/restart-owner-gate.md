# Restart Application — owner gate (installed release)

## Rule

Agents and CI must not mark File → Restart Application as fixed, done, or verified for the **installed release** path unless the owner has completed the Windows installed-build repro and this tree records attestation.

Linux CI green, coding-path supervisor relaunch, string sentinels, and PR merge are **not** that evidence.

## Two paths (do not conflate)

| Path | How Restart works | What proves it |
|------|-------------------|----------------|
| Coding | `restart.token` + `start-finos-supervisor.bat` → `start-finos-dev.bat` | Windows coding stack with supervisor |
| Installed release | release `app_restart` → `close_for_shutdown` → `spawn_installed_release_relaunch` (Start-Process current exe) → destroy → `app.exit(0)` — **not** `app.restart()` alone | Owner on `%LOCALAPPDATA%\finos\finos-desktop.exe` after NSIS install |

## Static anti-regression (not owner proof)

`installed_release_restart_must_spawn_exe_before_exit` in `crates/golden-harness/tests/desktop_menu.rs` must stay green. It only proves the spawn-before-exit / no-`app.restart()` pattern is present in `lib.rs` text.

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

- spawn-before-exit static test fails, or
- catalog `lastVerified` is a calendar date without `OWNER_CONFIRMED_INSTALLED_RESTART: true` in the attestation file.
