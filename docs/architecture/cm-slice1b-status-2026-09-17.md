# Cash Management Slice 1b — status for external review

**Date:** 17 September 2026  
**Product:** finos desktop (Profile A, local SQLite)  
**Area:** weekly capture on Cash Management (`TrendsCapture.tsx`)  
**Command:** `WeekCaptureAccept` (Slice 1 recon kept)  
**Not in this slice:** Slice 2; no rewrite of `CashManagement.tsx`

---

## Verdict

**Slice 1b landed.** The one-account wizard is gone. After the week dropdown the owner sees one table with all six accounts and Accepts on that screen.

A later owner requirement is now part of 1b (see **G7** below): cash may be left blank.

---

## What 1b asked (original pack)

Delete Week → Income → Roth → Speculation → Health → Car → 9 → Review.

After the week dropdown: one table, six rows visible at once — Income, FI Roth, Speculation, Health, Car, Account 9. Columns **Total Balance** and **Cash Balance**. Account 9 also types ETF total; 70% is read-only × 0.70. Accept on that screen. Keep `WeekCaptureAccept` + cash recon from Slice 1 / PR #10. Edit must refill the same table, not blank it.

**10-second check:** open capture. If all six accounts show without Next between accounts, 1b landed. If the owner still walks one account at a time, reject.

---

## Gate results

| ID | Requirement | Result |
|----|-------------|--------|
| G1 | Six rows at once; no Next from Income to Roth | **Pass** |
| G2 | Typed ETF 10000 → 70% is 7000 | **Pass** |
| G3 | Blank ETF is — , not $0 | **Pass** |
| G4 | Edit keeps values on the same table | **Pass** |
| G5 | No Adjust when typed cash matches reference | **Pass** |
| G6 | Speculation is a row | **Pass** |
| **G7** | **Blank cash is allowed on input and Accept** | **Pass (new)** |

G1–G6 are source-locked in `golden-harness` `accessibility` (`g1_g6_slice1b_capture_grid_one_table`). Slice 1 recon tests (`t1` no Adjust on match, Car gap + fee, blank ETF stays 0) remain. G7 is locked in that same grid test plus `trends_capture::blank_cash_skips_update_and_does_not_adjust`.

---

## New requirement (add to 1b feedback)

**Owner catch-up:** several weeks were not tracked; cash is often unknown.

**Rule:** Cash Balance may be left empty on any account, on first entry and on Accept.

- Blank cash **skips that week’s cash update** for that account.
- Blank is **not** stored as $0.
- Blank does **not** raise Adjust / recon.
- If the account already has cash from an earlier save, that number stays.
- Total Balance is still required for Accept.
- Type cash only when the owner knows it. The field placeholder is `skip`.

This applies to the same grid (no second cash screen).

Please treat G7 as an official 1b lock going forward, not a Slice 2 item.

---

## What reviewers should still not do

- Do not require Next between accounts.
- Do not treat blank cash or blank ETF as $0.
- Do not start Slice 2 from this status.
- Do not rewrite `CashManagement.tsx` for 1b.

---

## Post-1b

Week income was split after this status (Planned vs Reported). See [cm-post-1b-update-2026-09-17.md](cm-post-1b-update-2026-09-17.md).

## Adjacent (not 1b)

DeclarationRefresh on desktop open runs **once per local calendar date**. Manual Retrieve declarations still force-runs.
