# Execution board

Single place the agent reads so work proceeds step by step. Locked roadmap is V1.1 §12. Gate details stay in `m1-macos-gate.md` … `m6-gate.md`.

**Now:** none — M6 Windows slices 1–5 are green (AI Gateway advisory-only)  
**Next:** locked — name **unlock Postgres** / **OIDC** / **signed installers** to start those  
**Parked:** M1 macOS restore/handoff (needs a Mac)  
**Not started:** Postgres, OIDC, signed installers

## Loop

1. Do the current numbered step.
2. Run its **Pass** command.
3. If pass → start the next number immediately.
4. If fail → stop and fix. Never regenerate MAGI oracles.
5. When the slice’s last step passes → update Now/Next above and start the new Now.

Stop only for golden-oracle owner approval, a Mac-only step on Windows, or a milestone the board marks **Not started**.

## Done (Windows)

| Gate | Evidence |
|------|----------|
| M0–M4 | Root README checkboxes; `m2`–`m4` gate docs |
| M5 MAGI pack | `cargo test -p golden-harness --features magi-gate` — `m5_magi_pack_must_pass` |
| M5 G10-ADJ-1 | Oracle `G-MAGI-10.yaml`: 5200000 actual, SAFE, `warnings: []` |
| M5 Plan/burndown | `cargo test -p golden-harness -- plan_approve_does_not_rewrite_dividend_actuals` |
| M6 slice 1 Allocation | `cargo test -p golden-harness -- allocation`; schema 7 |
| M6 slice 2 Shopping Cart | `cargo test -p golden-harness -- cart`; schema 8 |
| M6 slice 3 Backtesting | `cargo test -p golden-harness -- backtest`; schema 9 |
| M6 slice 4 Classification review | `cargo test -p golden-harness -- classification`; schema 10 |
| M6 slice 5 AI Gateway | `cargo test -p golden-harness -- ai`; schema 11 |

## Stopped here on purpose

Postgres, OIDC, and signed installers are **Not started**. Live Grok calls need `XAI_API_KEY` (or `GROK_API_KEY`) in the environment or a gitignored `.env`. Do not paste the key into chat. Tests use `StubAdvisory` and do not need a key.

## How you use this

Keep working in this repo. The agent reads this file and continues **Now**. Name a different item only if you want to park M6.
