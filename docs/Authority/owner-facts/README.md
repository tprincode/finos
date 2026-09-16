# Owner facts (standing interview ledger)

Per-symbol **owner-provided** collector / ROC facts that agents must read **before** asking the owner again.

## Layout

```
docs/Authority/owner-facts/<SYMBOL>.md
```

Example: [`YBTC.md`](YBTC.md).

## Rules

1. If `<SYMBOL>.md` exists and marks a field locked → **do not re-interview** that field.
2. Ask only for fields marked **OPEN**, or when a live parsed 19a-1 % differs from stored plan (raise `roc_pct_change`; do not invent DB rows from chat).
3. These files do **not** auto-write SQLite. Seed / `RocPlanConfirm` / ticket Accept remain the writers of plan %.
4. Project Agent Store copies are secondary; the **git tip** file is authoritative for agents on this repo.

See also dated collector packs and snapshots in the parent `docs/Authority/` folder (HAKY live facts JSON, gap returns). Prefer promoting durable answers into `owner-facts/<SYMBOL>.md` instead of leaving them only in chat.
