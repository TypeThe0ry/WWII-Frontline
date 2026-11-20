# Gallery and HUD performance checks

Run `pnpm -C client exec playwright test -c playwright.ui.config.ts ui-gallery-perf.spec.ts`.
The spec prints JSON prefixed with `[ui-perf:...]` and attaches it to the Playwright result.
It uses one 1920x1080 headless Chromium page per test with `--disable-gpu`.

## DOM and repaint

The gallery and mounted HUD counts include all descendant **elements** of `[data-ui-gallery]`
and `[data-hud]`, respectively, including hidden descendants but excluding the root, text
and comments. The readouts sample scrolls to the middle of the 1,000-entry list. The HUD
fixture uses `createHudStore`, a frozen snapshot, eight squad members, and the real `Hud`.
The same mount transitions to spectating; the empty baseline counts its host **plus** all
descendant elements. This is the HUD-only baseline, not the full match shell.

For each scene Chromium tracing records `devtools.timeline` and `blink.user_timing`.
After fonts are ready and six animation frames have elapsed, the page marks a 60-frame
idle window. The test counts `Paint` events on the marking renderer thread inside that
window. Zero is the local settled-screen regression budget, not a budget for interaction
or animation. Opening a native dialog in a separate four-frame window must produce at
least one `Paint` on the same thread. Missing marks, trace data loss, or an undetected
positive control fail the test. Paint counts are page-wide, not per-node; they do not
measure rasterization, presentation, compositor-only changes, or paint duration.

## Signal workload

Render `<GalleryUiPerf />` inside `[data-ui-gallery]`. Only `/ui-gallery?uiperf=1` in dev
runs the one-shot probe. A hidden, document-connected subtree contains 100 separate Preact
components, each reading its own signal and rendering one span. Each rAF changes all 100
values in one batch and synchronously flushes Preact. `performance.now()` brackets the
batch and flush; all 100 committed text values are checked **after** the timer stops on
every frame. Drop 30 warmup frames, average 120 measured frames, and report nearest-rank
P95. The gate is the **unrounded mean <= 0.8 ms**, not a P95 or worst-frame gate. The DOM
readout is sampled once after removing the workload, and includes the output element.

This is a synthetic script-to-DOM benchmark mounted in the dev gallery. It excludes
initial mount, verification, rAF wait, style, layout, paint, GPU and real match updates.
It does not establish whole-app frame time or reference-hardware performance.

## Local sample (2026-11-20)

Focused Playwright run against Vite, Chrome for Testing headless shell 153.0.8010.12:

| Scene | Elements / limit | Idle Paints / 60 frames | Positive-control Paints |
| --- | ---: | ---: | ---: |
| Gallery, controls | 62 / 1500 | 0 | 4 |
| Gallery, readouts mid-list | 65 / 1500 | 0 | 6 |
| Mounted HUD | 74 / 350 | 0 | 4 |
| Empty HUD, including host | 1 / 20 | 0 | 4 |

Signal workload: **0.529 ms mean**, **0.800 ms P95**, 120 measured frames, 100 verified
changes per frame. Gallery descendants with the completed readout: 63. These are local
samples, not golden values; only the stated budgets are enforced.
