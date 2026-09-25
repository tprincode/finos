# App.tsx is the host, not the product

Measured 20 Sep 2026 after Collectors extract: `apps/desktop/src/App.tsx` is **12,986 lines / 489 KiB**. Vite Babel’s 500KB note is gone (threshold is 512 KiB). Further extracts still required — this file is still the host plus remaining screens.

## Lock

`App.tsx` may keep:

- `LocalTauriFinanceClient`
- in-app / native menu and `setScreen`
- `leaveWithoutSaving` / Restart / snapshot
- open-path refresh bars (`HomeOpenGet`, last-price, collectors clock)

It may not gain another screen, table, or wizard. New UI goes in `apps/desktop/src/features/<name>/`.

## Already out

Shopping Cart, graphing, cash Register / Elements / YTD / Coverage / Week desk / Week Ahead, Home Dividend Plan, cash-flow chart, pickers, Import wizard window, **Collectors** + Reevaluate (`features/collectors/`), **Income Plan** week + grid (`features/income-plan/IncomePlanScreen.tsx`). App mounts those screens. Run/refresh/ticket commands and Income Plan queries stay on the host. `PlanHorizonPrompt` stays in the same folder and still mounts on System update tasks.

## Still in App.tsx (extract in this order)

1. **Position Details** / Add Position / Add Lot
2. **Home** remainder (cards, tickets button, refresh)
3. Settings / Tickets / Calculator / Dashboard chrome

## Golden rule

A golden that today `read_to_string(App.tsx)` for a screen must read the feature file after that extract. Do not copy markup so both files stay huge. Do not change owner behavior in the same slice as a move.

## Pass

- After Collectors: App.tsx below 500KB (Babel note gone) or prove the remaining bytes are host-only.
- After the list: App.tsx is a thin host (target **under 200KB**).
- `cargo test -p golden-harness --test accessibility --test collectors --test income_plan_display --test account_value --test desktop_menu`
