# Installed-release Restart — owner attestation

Status for agents and CI. Change **only** the values inside the STATUS fence. Do not set confirmed true unless the owner completed the Windows installed-build File → Restart repro on this tip (or a named successor SHA).

## STATUS

```
OWNER_CONFIRMED_INSTALLED_RESTART: false
ATTESTED_TIP_SHA: none
ATTESTED_DATE: none
NOTES: Awaiting owner Windows NSIS / finos-desktop.exe File → Restart pass after spawn-before-exit relaunch. Static CI is not proof.
```

When the owner confirms, edit STATUS to confirmed true, tip SHA, date (YYYY-MM-DD), and a one-line note (Desktop shortcut or start-finos-installed.bat; new PID observed). Then set `file-restart-graceful.lastVerified` in `core-functions.json` to that date.
