# M2-020 transport fixtures (2026-11-20)

Shared EPYC workstation, PI-109, not reference hardware. The runner and
reproduction commands are under [scripts/netem/m2-020](../scripts/netem/m2-020/README.md).
No test build was initiated by the harness. All successful runs used the
prebuilt release pair below (the server's startup Git SHA does not describe the
uncommitted regulator changes; the binary digests distinguish this pair):

- Server SHA-256 `e478ba00fca986a12903aaf4e7079e0e10600c0e39c7489317a811e41de69123`
- Headless SHA-256 `f3b9f3b18f6646729a826c17f7e0bc994ae26c1cc2c9900d972d292c8fbffaec`
- Protocol `4`, startup data hash `2637b94dda28a192`.

## M0 short matrix

Each case used a real server and headless transport in its own loopback network
namespace. All eight exited 0, with no failed joins, disconnects, decode errors, or
missing baselines. `wire` is the server's estimated downstream kbps histogram,
using the delta of sum/count over the measurement window. Drops include the
lead-in but do not include traffic on the unshaped metrics listener.

| Profile | Transport | Connected + bots | Window (s) | Server wire mean (kbps) | `wire` samples | `netem` drops down/up |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Typical | WT | 20 + 40 | 10.001 | 33.134 | 200 | 624 / 615 |
| Typical | WS | 20 + 40 | 10.002 | 28.184 | 200 | 503 / 519 |
| Middle | WT | 20 + 40 | 10.001 | 35.911 | 200 | 194 / 207 |
| Middle | WS | 20 + 40 | 10.001 | 31.630 | 199 | 164 / 175 |
| Weak | WT | 1 + 12 | 15.002 | 14.766 | 15 | 32 / 30 |
| Weak | WS | 1 + 12 | 15.002 | 11.484 | 15 | 28 / 28 |
| Clear | WT | 2 + 12 | 8.001 | 16.960 | 16 | not shaped |
| Clear | WS | 2 + 12 | 8.002 | 12.651 | 16 | not shaped |

- Middle: each direction configured 2% loss and 50 ms fixed delay. Whole-run
  headless RTT P50/P99 was 100.330/267.177 ms WT and 100.266/266.571 ms WS.
- Weak: the `le=0.400001` scale bucket added 30/30 WT and 21/23 WS evaluations;
  `le=0.399` stayed at zero and `le=1` covered all evaluations in both windows.
  These buckets are not exact minimum measurements. The first post-removal
  scrape with `net_regulated_clients=0` came at +9.056 s WT and +8.041 s WS;
  both stayed at 0 through the last scrape (about +12 s). This samples recovery
  to scale 1 with one client; it does not pinpoint the transition or provide a
  phase-specific headless RTT distribution. The 150 ms base RTT and 80 kbit/s
  cap are commanded netem settings, not measured RTT or throughput.
- Clear: every observed scale evaluation was 1 (WT sum/count 32/32, WS 24/24;
  all evaluations at most 1), with no regulated clients. No byte-for-byte
  comparison against M1-024 was made in the fixture.
- Tool checks: 4/4 Python tests passed, including live UDP/TCP filter checks
  (including both middle directions); Ruff 0.14.14 passed. `shellcheck` and
  `bash -n` passed for the staging script. Carentan baking and
  SHA-256 verification passed for all 380 raw FCOL chunks.

## Acceptance scope

Bot/connection counts are not post-AOI visibility or tier counts. The dense
fixture has no controlled 150 unique generated events/s or specified vehicle
load. A headless `events` aggregate counts observations across clients, not
generation. The exact §6.9.2 and §6.9.3 loads, retained-T3 P99 on real transport,
reference hardware, and the CPU benchmark are unmeasured by this report. The
remaining byte-for-byte and HUD limit checks belong to the implementation tests.

## Carentan M1 concurrent soaks

Both processes loaded 380 raw FCOL chunks. The staged chunk manifest SHA-256 was
`491d4c9ce03c2c239cf657e5841085b322bd727c2298d73456cbc401be0225e8`.
The two matches ran in separate network namespaces. The logical CPU assignments
below were checked against [live thread masks](m2-020-bwreg-affinity.tsv). The
workstation remained shared; these assignments are not host CPU reservations.

| Fixture | Sim | Other server threads | Headless | SMT siblings |
| --- | --- | --- | --- | --- |
| Typical WT | 32 | 36,39,42 | 43,45 | 80,84,87,90,91,93 |
| Dense WS | 34 | 24,29,41 | 6,10 | 54,58,72,77,82,89 |

Both runners exited 0. Each had 180/180 measurement scrapes at the configured
population, all 20 HQ deploys before the first measurement (latest spawn tick
73 WT / 32 WS, start tick count 374 WT / 341 WS), and shortest client lifetime
1840.031 s in each case. Failed joins, disconnects, decode errors, and missing
baselines were all zero. [Compact evidence](m2-020-bwreg-fixtures.jsonl) includes
the endpoint histogram buckets and per-client reports for all eight smokes and
both soaks.

| Fixture | Clients + bots | Window (s) | Server wire samples | Wire mean (kbps) | Wire P99 bucket (kbps) | Headless P99 (kbps) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Typical WT, 5% netem | 20 + 40 | 1800.001 | 35,880 | 21.428 | (32, 54] | 31.736 |
| Dense WS, unshaped | 20 + 80 | 1800.002 | 35,615 | 26.042 | (32, 54] | 42.136 |

- Typical WT: downstream passed 1,245,917 packets and dropped 65,603 (5.002%);
  upstream passed 1,321,956 and dropped 69,954 (5.026%) at the end of the window.
  The headless whole-run RTT P50/P99 was 82.553/256.720 ms.
- Both had 0 `aoi_starved_total` increments in the measurement window; neither
  establishes the retained-T3 timing gate. Every measured server wire sample
  was at most 400 kbps. The 21.428 kbps typical mean is below 68 kbps for this
  workload, not evidence of 50 post-AOI visible players per observer.
- The dense server's whole-run tally recorded 595 shots, 193 hits, 105 kills,
  and 105 spawns: 998 counted gameplay events over 1841.398 s (0.542/s for that
  subset, without double-counting headshots). This tally does not enumerate all
  snapshot events; neither it nor the 12,168 headless event receipts establish
  the 150 events/s workload. The P99 result is a dense-population fixture only.
- Whole-run server tick P99 was 13.042 ms typical and 32.235 ms dense; 90/479
  overruns, with longest runs of 2/8 consecutive overruns, respectively. These
  are not passing tick-budget runs and do not isolate regulator CPU cost.
