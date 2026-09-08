# M6 gate — Decision intelligence (slices 1–5)

V1.1 §12: Allocation, Shopping Cart, backtesting, classification review, AI advisory. **AI posting of facts is forbidden (ADR-0012).**

Allocation, cart, backtest, classification review, and `AiAnalyze` must not post ledger, lots, or MAGI facts, and must not rewrite MAGI oracles.

## Commands

```
cargo test -p financial-domain -- allocation
cargo test -p golden-harness -- allocation
cargo test -p financial-domain -- cart
cargo test -p golden-harness -- cart
cargo test -p financial-domain -- backtest
cargo test -p golden-harness -- backtest
cargo test -p financial-domain -- classification
cargo test -p golden-harness -- classification
cargo test -p financial-domain -- advisory
cargo test -p golden-harness -- ai
cargo test --workspace
```

## Slice 1 must pass

| Filter | What it proves |
|--------|----------------|
| `allocation_target_does_not_change_cash` | L0: target is not cash |
| `allocation_target_does_not_post_dividend_facts` | L2: 60.00% target; DividendGet unchanged |

## Slice 2 must pass

| Filter | What it proves |
|--------|----------------|
| `cart_line_is_not_a_fill` | L0: cart quantity is not a lot fill |
| `cart_item_does_not_post_dividend_or_lot_facts` | L2: CartGet returns the line; DividendGet and BasisGet unchanged; remove clears it |

## Slice 3 must pass

| Filter | What it proves |
|--------|----------------|
| `backtest_run_is_not_a_posted_activity` | L0: hypothetical P&L is not cash |
| `backtest_run_does_not_post_dividend_facts` | L2: BacktestGet returns the run; DividendGet unchanged |

## Slice 4 must pass

| Filter | What it proves |
|--------|----------------|
| `classification_review_does_not_change_magi_oracle` | L0: review does not change MAGI decision |
| `classification_review_does_not_post_dividend_or_rewrite_oracles` | L2: ClassificationReviewGet returns the review; DividendGet unchanged |

## Slice 5 must pass

| Filter | What it proves |
|--------|----------------|
| `advisory_is_not_a_posted_activity` | L0: a recommendation is not a ledger post |
| `secrets_are_stripped_from_prompts` | L0: API keys are redacted before prompts |
| `ai_analyze_does_not_post_dividend_facts` | L2: stub AiAnalyze; DividendGet unchanged; key not stored |

Live Grok is desktop-only: set `XAI_API_KEY` or `GROK_API_KEY`. Tests use `StubAdvisory` and must not call xAI.

## Out of these slices

Postgres, OIDC, signed installers.
