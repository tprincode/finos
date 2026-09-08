# Position hub gap audit and resolution

Status: slices A–D landed. E (fix thin collectors until gate green) remains.

## Surfaces

| Surface | Role | Gap |
|---|---|---|
| Collectors → symbol | Ops only | ~~Still shows mini dossier~~ → runs/payload + Open PD |
| Position Details → symbol | Full hub | Decl depth gate + Received panel in; Settings templates still on PD |

## Gap inventory

| ID | Gap | Fix slice | Status |
|---|---|---|---|
| G1 | Adapter functional only if ≥12 paid decls (unless too new) | C | Done — `declaration_lookback_short` miss |
| G2 | Unlimited store over time; display starts at 12 | B/C | Done — labels + gate |
| G3 | No per-year / all-years received panel | B | Done — Received payments |
| G4 | `derived_walk` / walk on screen | A | Done — owner labels |
| G5 | Template edit on PD | D | Done — Settings table; PD read-only |
| G6 | Collectors mini position page | A | Done — ops only |

## Design locks

1. Adapter gate: retrieve first. If paid ≥ 12 → ok (inception N/A). If paid &lt; 12 → optional inception may confirm complete short history; otherwise miss.
2. Storage: all non-superseded decls forever; retrieve all vendor rows; gate requires ≥12 unless inception-short.
3. Face source = adapter name, never walk policy.
4. Template knobs on Settings; inception optional (rare too-new names).
5. Received: all-years total + per calendar year table.

## Remaining

### E — Fix non-functional collectors
- After `npm run desktop`, Run misses only until empty.
- For newer names under 12 paid: set optional **Inception** on Settings so the post-retrieve check can accept the short complete set.

## Pass

XDTE-class symbols show Calculator, Plan, accounts, ≥12 decls or short-history message, pay summary/dates, per-year + all-years received; no walk as source; Collectors ≠ thin PD; templates in Settings.
