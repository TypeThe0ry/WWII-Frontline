# M2-020 #3: floor-scale T3 delivery sidecar (2026-11-20)

**PI-109 shared EPYC workstation, not reference hardware.** Both real WebTransport
Carentan runs met the 600 ms P99 and 0.5/min starvation bounds for the observed
friendly T3 cohort. The shipped trace does not expose continuous scheduler
admission or Remove records, so this is a real delivery proxy, not exact
retained-set acceptance. The directly instrumented scheduler fixture is
reported separately in [the main report](m2-bandwidth-regulator.md).

## Carentan results

One stock headless receiver, 24 bots, private loopback netns per run, 75 ms
configured one-way delay in both directions and downstream 80 kbit/s shaping.
Actual loss is the drop delta divided by the delta of released, dropped and
queued packets, using interior qdisc scrapes in the stated floor window.

| | 5% loss configured both ways | 18% loss configured uplink only |
| --- | ---: | ---: |
| Connected time | 88.033 s | 100.032 s |
| Contiguous floor ticks used, inclusive (30 Hz) | 1435-2695 | 184-3079 |
| Interior starvation scrape span | 41.682 s | 96.257 s |
| Downstream qdisc loss | 60/1,292 (4.644%) | 0/3,195 (0%) |
| Upstream qdisc loss | 67/1,293 (5.182%) | 623/3,538 (17.609%) |
| Offered C2 snapshots not decoded in floor window | 50/841 | 0/1,931 |
| Distinct friendly T3 tracks / adjacent intervals | 4 / 194 | 4 / 301 |
| Server tick gap P99 / max | 500.0 / 1,333.3 ms | 500.0 / 733.3 ms |
| Headless arrival gap P99 / max | 516.1 / 1,334.6 ms | 532.7 / 729.9 ms |
| Above 600 ms, by either measure | 1/194 | 1/301 |
| `aoi_starved_total` delta / rate | 0 / 0 per min | 0 / 0 per min |
| `net_far_shed_total` delta in same scrape span | 6,664 | 13,797 |

Both runs joined and deployed once, exited 0, and had zero disconnects,
deaths, respawns, reordered snapshots, decode errors and missing baselines
at the receiver. Server-wide: 8 shots, no hits/kills/spawns in either run;
0 and 2 skipped wall-clock slots, respectively. The two directions were
shaped independently. In the first run, 89/194 intervals crossed at least
one offered C2 absent from the decode trace; its longest gap (entity 113,
ticks 1990-2030) crossed two. This run combines scheduling/admission and
transport delivery. As a diagnostic only, its other 105 intervals had 500.0 ms
tick P99 and 516.1 ms arrival P99. In the uplink-only run all offered C2 in
the floor span decoded, so the longest gap (entity 113, ticks 1996-2018)
is not explained by downstream C2 loss. It could include a scheduler
admission transition that this trace cannot see.

## Cohort and floor reconstruction

- Joined `[aoi-wire]` (one row of actual decoded C2 player updates, not a
  reconstructed cache) to `[aoi-pose]` on receiver ID and tick. Selected
  all same-team player records in `192.05 < distance_m < 399.95`, with 5 cm
  guards for position quantization. Same-team records avoid enemy cone/LoS
  transitions. The deployed receiver did not change life state, and the
  server saw no kills. Split a track at any observed team/tier departure,
  and at any non-floor span; never bridge those explicit transitions. No
  cadence, gap-length, or intervening-loss filter was applied to the main
  sample. A target never delivered, or a cut/readmission or tier excursion
  between delivered records, cannot be classified from these logs.
- With exactly one receiver, scrape `net_budget_scale` at about 5 Hz.
  Verified zero `le=0.399` increments; consecutive evaluations counted in
  `le=0.400001` are the current config's 0.40 band. Each evaluation holds
  until the next one. Counted from the first scheduler pose after deployment
  to align every tenth offered C2 with the corresponding histogram
  increment: scrapes began at count zero, increments were isolated, the
  pose sequence matched every expected WT send slot, and the evaluation
  timestamps agreed within one 50 ms stderr flush. Kept spans between
  first and last confirmed consecutive floor evaluations, without
  extrapolation. Only one such span in each Carentan run yielded T3 pairs;
  whole-run floor evaluation counts were 146/176 and 194/200 respectively.
- Adjacent records were compared per receiver/entity/segment, using original
  C2 ticks and monotonic line-read timestamps on headless stderr. The latter
  includes trace/pipe and workstation scheduling latency, not a hardware
  packet timestamp. P99 uses nearest rank `ceil(0.99 * n)`.

| Entity ID | Two-way loss: first-last tick, records | Uplink-only loss: first-last tick, records |
| --- | --- | --- |
| 107 | 2005-2678, 46 | 2011-3070, 72 |
| 111 | 1804-2686, 60 | 1807-3076, 86 |
| 113 | 1930-2674, 49 | 1921-3077, 78 |
| 115 | 2050-2678, 43 | 2063-3077, 69 |

## Reproduction and scope

- Reused `target/release/{frontline-server,headless-client}`, with no build.
  Server SHA-256 `26021057ed7fbe444b030df278f9bb62e21db4722465236ac034e181c010d720`;
  headless SHA-256 `f3b9f3b18f6646729a826c17f7e0bc994ae26c1cc2c9900d972d292c8fbffaec`.
  Startup reported protocol 4 and data hash `2637b94dda28a192` (the disposable
  patrol edits below are fixture data, not covered by that reported hash).
- Launch in `unshare -n`, `ip link set lo up`, `FL_AOI_TRACE=1`; use
  [the fixture runner's `shape`](../scripts/netem/m2-020/run.py) for separate
  UDP source/destination port bands (seeds 27/28). Settings: downstream
  `loss 5% delay 75ms rate 80kbit` / uplink `loss 5% delay 75ms` for the
  first run; downstream `delay 75ms rate 80kbit` / uplink `loss 18% delay
  75ms` for the second. No change to the shared harness or server settings.
- Used the Carentan collision fixture (380 raw FCOL chunks, all matched its
  manifest), staged privately with two different `spawns.json` copies.
  Only `patrol_points` changed: a single server waypoint `[735,1580]` and a
  single headless waypoint `[476,1600]`. This creates a friendly group moving
  away from the receiver without dragging it out of HQ. Server flags:
  `--map carentan/m1 --mode assault --bots 24 --dev-cert --ws-port 0 --wt-port 0 --metrics-port 0 --pin-sim-core 12`;
  headless: `--map carentan/m1 --transport wt --deploy-hq --count 1 --workers 1 --seed 26 --warmup 5 --duration 88` (second run: 100).
  Pass each copy via its own `--data-dir` and the local WT listener and cert
  hash to headless. Carentan server/client PIDs were 526674/526769 and
  552711/552736, pinned to CPU sets 12-13 and 14 respectively.
- The initial M0 probe (PIDs 489204/489224, 80 s, 40 bots, two-way loss)
  produced only one track entering T3 late, with 11 floor intervals, 500.0 ms
  tick P99 and 502.9 ms arrival P99. It is not pooled with Carentan. All
  sidecar processes exited and the disposable data and traces were removed.

Not measured in the proxy runs above: the server's exact retained entity set between updates,
enemy or vehicle T3 cadence, WS T3 cadence, and reference hardware. Exact
headless #3 validation needs a server-side per-C2 admission/identity trace
joined with the client trace so that captures can exclude true admission and
life transitions without selecting tracks by their measured cadence.

## Exact retained-set follow-up: uplink-only WT

**M2-020 #3 thresholds met for continuously retained friendly player T3 in
this PI-109 Carentan fixture.** The instrumented 100.032 s replication had
299 adjacent decoded updates across four player slots and five continuous
retention segments at `scale=0.40`. Server-tick P99/max were both **500.0 ms**;
headless monotonic arrival P99/max were **503.1/503.2 ms** (nearest rank).
No interval exceeded 600 ms. `aoi_starved_total` stayed unchanged over the
84.177 s interior scrape span (**0/min**); `net_far_shed_total` increased by
13,464 in that span. Measured qdisc loss was **0/2,709 down, 464/2,597 up
(17.867%)**. Enemy, vehicle, WS, and reference-hardware cadence remain unmeasured.

Joined `[aoi-t3] PLAYER TICK SCALE LO HI` to `[aoi-pose]` and `[aoi-wire]`
by receiver and C2 tick. The post-trim mask gives exact selected friendly
player slots (`192 < dist <= 400 m`), before encoding. Compared adjacent
decoded updates only within uninterrupted runs of set bits at 0.40; an
absent bit, non-floor value, or missing expected trace row breaks a run,
regardless of the gap length. Entity 113, for example, was absent on offers
2014, 2015, 2017 and 2018, so its two segments were not bridged. There were
no kills/respawns, skipped simulation slots, or reduced-cadence ticks.

All 2,003 expected WT snapshot offers had one row in each server trace,
and the logger reported no output loss. The early floor window (ticks
403-536) contained no T3 pairs. In the measured floor window, every offer
from tick 553 through the last received tick 3088 decoded (1,691/1,691).
The three subsequent offers (3089, 3091, 3092) had no decode before client
exit and are right-censored; no measured interval spans a missing C2.
Headless reported zero disconnects, reorders, decode errors and missing
baselines. Both processes exited 0.

Repro: same staged patrols, CPUs, headless release binary, and uplink-only
100 s recipe above, with `FL_AOI_TRACE=1` on both processes and
`FL_AOI_REGULATOR_TRACE=1` on the server. Reused the new prebuilt server
(SHA-256 `b756b651c90b6ff9d746de45bc128a2c1dff8d3f8c053f0a8beac0794ba0b1c8`)
and let it exit at 105 s. Server/client PIDs 714666/714890; the namespace,
staged data, and traces were temporary.
