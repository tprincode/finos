# Screen Atlas

Documentation / human eyeball captures of every desktop menu surface.

**Not a CI pixel golden.** Automated UI regression stays on:

```bat
cargo test -p golden-harness
```

(source/query goldens such as `accessibility`, `ui_modules`, `desktop_menu`).

## How to run

1. Start the desktop (`finos.bat` / coding launch).
2. **Tools → Component Registry → Capture All**. Capture Page walks one page.
3. Or drop a token and restart so the app auto-runs:

```bat
echo run> "%LOCALAPPDATA%\com.finos.desktop\screen-atlas.run"
```

Then restart the desktop (or write `restart.token` as usual). About 2.5s after host start, Rust emits `finos-screen-atlas` (and deletes the token). The UI claims the event once and starts the atlas.

## Output

`%LOCALAPPDATA%\com.finos.desktop\evidence\screen-atlas\YYYY-MM-DD\`

- One PNG per target (`home.png`, `cm-weekly.png`, …)
- `index.md` — menu path, status, pinned asOf

On **Tools → Component Registry**:

- **Open folder in Explorer** (or click the folder path) opens that directory in Windows Explorer so you can browse/preview PNGs with the OS viewer.
- The **Viewer** lists PNG names and loads **one** image at a time on click (avoids webview OOM from bulk base64).

Copy PNGs into this folder only if you want them in git for a doc pack; by default they stay in LocalAppData evidence (operational data).

## Freeze mode (docs determinism)

While capturing:

- asOf pinned for the session
- LastPrice refresh and declaration fleet skipped
- ECharts `animation: false` on shared chart option builders
- CSS expands nested `overflow: auto` panes for full-height capture
- **Light color scheme forced** (avoids OS dark mode + incomplete dark CSS → dark-on-dark)
- Waits for the orange **Page activity** chip (`aria-busy`) to go idle, App busy to clear, and **all** `Loading…` copy (including nested “Loading dividend weeks…”) to stay clear for ~1.5s, then a per-screen settle delay
- Expands nested `overflow` / `max-height` and captures **full scroll height** of `#root` (not only the visible window), width capped to the viewport for readable sheets

Still uses the **owner live book** — a snapshot of this machine, not a sealed fixture oracle.

## Targets

Defined in [`apps/desktop/src/features/screen-atlas/atlasTargets.ts`](../../apps/desktop/src/features/screen-atlas/atlasTargets.ts). Tools → Components, this capture list, and the data-snapshot workbook `Template_UiModules.xlsx` are one screen set: every App `Screen` and Cash Management desk. A missing row is added. A live screen is not removed to force a match.
