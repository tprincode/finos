import {
  Callout,
  Divider,
  Grid,
  H1,
  H2,
  H3,
  Pill,
  Stack,
  Stat,
  Table,
  Text,
  UsageBar,
} from "cursor/canvas";

/**
 * Shopping Cart design after 10 Sep reviews.
 * Leftover cash still earns. Feature folder, not App.tsx.
 * Parked until the owner names SC-1.
 */
export default function ShoppingCartDesign() {
  return (
    <Stack gap={20}>
      <Stack gap={6}>
        <H1>Shopping Cart — leftover keeps yield</H1>
        <Text tone="secondary">
          10 Sep 2026 reviews accepted. Authority v1.0 locked 14 Aug 2026.
          Board: Parked until you name SC-1. Design only — no cart commands.
        </Text>
      </Stack>

      <Grid columns={4} gap={12}>
        <Stat value="+$7.12" label="Example B net annual (not +$6.92)" tone="success" />
        <Stat value="$2.00" label="Surrendered on $59.90 spent" />
        <Stat value="$5.87" label="Leftover still earns ~$0.20" />
        <Stat value="Parked" label="Feature folder when named SC-1" />
      </Grid>

      <Callout tone="warning" title="Accepted math">
        Net income = buy plan minus plan on dollars spent. Leftover cash
        keeps the money-market yield. The old +$6.92 surrendered yield on
        the leftover $5.87. That overstates what you give up.
      </Callout>

      <H2>Step rail — same screen, not a hidden wizard</H2>
      <Table
        headers={["Step", "Owner action", "This example"]}
        rows={[
          ["Draft", "Pick account, named lots, researched buys", "current"],
          ["Evaluate", "Frozen snapshots; intent still visible if over remaining", "next"],
          ["Agree", "Freeze. Blocked if lots do not cover spend", "later"],
          ["Confirm sell", "ActivityPost + LotAssign on named lot", "later"],
          ["Open lot", "Add Lot prefilled; owner clicks Open lot", "later"],
        ]}
        rowTone={["info", "neutral", "neutral", "neutral", "neutral"]}
        striped
      />
      <Text size="small" tone="secondary">
        Status is a word: draft, agreed, executing, complete. Color is a cue
        only.
      </Text>

      <H2>Afford strip — FI Roth SPAXX covering lot</H2>
      <Text>
        Partial sell: spend $59.90 of live remaining $65.77. Goldens compare
        spend to live remaining, not a frozen Roth balance.
      </Text>
      <Table
        headers={["Named remaining", "Spend", "Leftover", "Afford"]}
        columnAlign={["right", "right", "right", "left"]}
        rows={[["$65.77", "$59.90", "$5.87", "Can afford — leftover still cash"]]}
        striped
      />
      <Table
        headers={["Named remaining", "Spend", "Leftover", "Afford"]}
        columnAlign={["right", "right", "right", "left"]}
        rows={[["$65.77", "$329.45", "—", "Cannot afford — $340 intent, Agree blocked"]]}
        rowTone={["warning"]}
        striped
      />

      <H2>Income keep vs swap — leftover-keeps-yield</H2>
      <Table
        headers={["Line", "Qty / $", "Annual $", "Monthly $", "Weekly $"]}
        columnAlign={["left", "right", "right", "right", "right"]}
        rows={[
          ["Keep $65.77 SPAXX", "$65.77", "2.20", "0.18", "0.04"],
          ["Surrender spent $59.90", "$59.90", "−2.00", "−0.17", "−0.04"],
          ["Leftover $5.87 still earns", "$5.87", "0.20", "0.02", "0.00"],
          ["Buy 2 HAKY @ $29.95", "2 sh", "+9.12", "+0.76", "+0.18"],
          ["Net vs keep", "—", "+7.12", "+0.59", "+0.14"],
        ]}
        rowTone={["neutral", "neutral", "info", "success", "success"]}
        striped
      />
      <Text size="small" tone="secondary">
        Source: live local.sqlite 10 Sep 2026, illustrative. SPAXX plan
        3.34%/yr. HAKY $0.38 × 12. Monthly = annual ÷ 12. Weekly = annual
        ÷ 52. HAKY is Core.
      </Text>

      <H3>$340 intent — leftover-keeps-yield, Agree blocked</H3>
      <Table
        headers={["Horizon", "Net vs keep $340"]}
        columnAlign={["left", "right"]}
        rows={[
          ["Annual (50.16 − 11.00 spent yield)", "+$39.16"],
          ["Monthly", "+$3.26"],
          ["Weekly", "+$0.75"],
          ["Named lot cover", "$65.77 of $340 — insufficient_lot_qty"],
        ]}
        striped
      />

      <H2>Mix bars — cash off the bar</H2>
      <Text>
        BR-SC-011. FI Roth invested $8,339 before. After 2 HAKY, Core
        +$59.90. Cash is a label, not a slice.
      </Text>
      <Stack gap={10}>
        <UsageBar
          total={8339}
          topLeftLabel="Before — invested $8,339 (SPAXX $65.77 off mix)"
          topRightLabel="F 43.0% · C 21.2% · R 35.8%"
          segments={[
            { id: "f0", value: 3586, color: "green" },
            { id: "c0", value: 1770, color: "blue" },
            { id: "r0", value: 2983, color: "orange" },
          ]}
        />
        <UsageBar
          total={8399}
          topLeftLabel="After 2 HAKY — invested $8,399 · leftover $5.87 off mix"
          topRightLabel="F 42.7% · C 21.8% · R 35.5%"
          segments={[
            { id: "f1", value: 3586, color: "green" },
            { id: "c1", value: 1830, color: "blue" },
            { id: "r1", value: 2983, color: "orange" },
          ]}
        />
      </Stack>

      <H2>Cash floors (three accounts only)</H2>
      <Table
        headers={["Account", "Floor", "Rule"]}
        columnAlign={["left", "right", "left"]}
        rows={[
          ["Income", "$5,000", "Warn if cash sell would drop below"],
          ["Car", "$4,000", "Warn; override needs a typed reason"],
          ["Health", "$200", "Warn; override needs a typed reason"],
          ["FI Roth / 9 / others", "none", "Do not invent a floor"],
        ]}
        striped
      />

      <Divider />

      <H2>Component map — reject fat-file patches</H2>
      <Table
        headers={["Layer", "Path", "Job"]}
        rows={[
          [
            "UI",
            "apps/desktop/src/features/shopping-cart/",
            "Screen, StepRail, AffordStrip, IncomeCompare, MixBars, TradeoffCallout, ExecutePanel",
          ],
          [
            "Pickers",
            "apps/desktop/src/features/shared/pickers/",
            "Extract account, lot, researched-symbol from App.tsx when SC-1 is named",
          ],
          [
            "Host",
            "crates/application-core/src/cart.rs",
            "Scenario, evaluate, agree — not queries.rs math",
          ],
          [
            "Domain / SQL",
            "financial-domain + storage-sqlite cart.rs",
            "Keep M6 modules; extend. Keep CartItemAdd until goldens replace it",
          ],
          [
            "App.tsx",
            "menu + setScreen",
            "No cart grid markup",
          ],
        ]}
        striped
      />
      <Callout tone="info" title="Pushback kept">
        There is no features folder today. Extract only the three pickers.
        Do not split the rest of App.tsx or move weekly actuals out of
        queries.rs. Host cart.rs is application-core; domain and storage
        cart files already exist.
      </Callout>

      <H2>Locked calculation rules</H2>
      <Table
        headers={["Output", "Rule"]}
        rows={[
          ["Income surrendered", "Plan on dollars spent only"],
          ["Leftover yield", "Still earns MM plan; stays off the mix bar"],
          ["Net annual", "Buy plan − surrendered on spend"],
          ["Monthly / weekly", "Annual ÷ 12 / ÷ 52"],
          ["Agree", "Blocked when spend > live remaining"],
          ["Cash floor", "Income 5000 / Car 4000 / Health 200 only"],
          ["Draft", "Zero writes to lots, ledger, Income Plan, ROI"],
          ["Evaluate", "Never LotOpen"],
        ]}
        striped
      />

      <Pill tone="neutral" active>
        Parked — name SC-1 to code
      </Pill>
    </Stack>
  );
}
