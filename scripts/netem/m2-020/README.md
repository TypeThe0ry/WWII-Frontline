# M2-020 real-transport fixtures

Run from `b6/bwreg`. Requires Linux/Python >= 3.11, `unshare`, `ip`, `tc` with
netem, prio and flower support, and permission to create a private network
namespace. Requires matching prebuilt `frontline-server` and `headless-client`
binaries (protocol and embedded data hash must agree). The scripts never build
Cargo or use the gateway. WT pins the server's throw-away certificate. Each run
uses an independent namespace and ephemeral listener ports; it stops only its
own logged child PIDs. Use separate server/client CPU sets for concurrent soaks
when possible; pinning does not reserve the cores from other workstation jobs.

For Carentan, the tracked tree has the chunk index, not the baked chunks. With
NumPy and the tracked Carentan maps present, run `data=$(scripts/netem/m2-020/prepare-carentan.sh)`.
This stages the map under `/data/scratch`, with the same two synthetic patrol
points as B6 Carentan. It does not inject the fleet from that report. For a
quick connectivity smoke use `--map m0 --data "$PWD/data"` with generated M0
data, `--seconds 5 --warmup 2 --count 2 --bots 2` (for `weak`, use `--count 1`).

```sh
harness=scripts/netem/m2-020/run.py
bin=/absolute/path/to/release
data=$(scripts/netem/m2-020/prepare-carentan.sh)
common=(--server "$bin/frontline-server" --headless "$bin/headless-client" --data "$data")

# Concurrent workload probes, each with a >=1,800 s measurement window.
"$harness" "${common[@]}" --profile typical --transport wt & typical=$!
"$harness" "${common[@]}" --profile dense --transport ws & dense=$!
wait "$typical"; typical_status=$?
wait "$dense"; dense_status=$?
printf 'typical=%s dense=%s\n' "$typical_status" "$dense_status"

# Repeat with --transport ws/wt respectively for both transports per workload.
"$harness" "${common[@]}" --profile weak --transport wt
```

| Profile | Default population | Duration | Network |
| --- | ---: | ---: | --- |
| `typical` | 20 sockets + 40 bots | 1,800 s | 5% loss, 40 ms +/- 20 ms one-way, on both directions |
| `middle` | 20 sockets + 40 bots | 10 s | 2% loss and 50 ms delay each way (100 ms configured base RTT) |
| `dense` | 20 sockets + 80 bots | 1,800 s | No injected impairment |
| `weak` | 1 socket + 12 bots | 60 s | 5% loss, 75 ms each way, downstream 80 kbit/s, then 20 s clear |
| `clear` | 20 sockets + 40 bots | 1,800 s | No injected impairment |

By design the 80 kbit/s cap is not shared across clients; use separate weak
runs for multiple clients. `/metrics` is excluded from shaping. The window
starts after all expected sessions are observed and a warm-up. The client
report, server report, startup protocol/data hash, binary hashes, both netem
queues, and timed `/metrics` samples are saved in the printed scratch path.
`summary.json` uses server histogram deltas for the mean and checks the exact
400 kbps bucket (rather than estimating a quantile within it). The server's
`wire` histogram is a line-rate estimate, not a capture of QUIC/TCP overhead.
Sampled full population plus uninterrupted client reports is a connectivity
check, not a measurement of visibility. Keep raw scale histograms and the
recovery timestamps for analysis; this fixture does not assert exact scale
minima, recovery within 10 s, or retained-T3 timing. Archive desired evidence
and remove only your printed test/staging paths from `/data/scratch` afterward.
Use `python3 -B scripts/netem/m2-020/collect-evidence.py <run-dir> ...` to emit
compact JSON Lines with selected endpoint scrapes, populations, client continuity,
queue counts, binary hashes, and (for weak runs) recovery samples.

**Acceptance limits:** PI-109 workstation, not reference hardware. Bot totals
do not establish 50 visible players per receiver or the 4/10/14/22 tier mix;
Carentan starts the teams at separate HQs. Likewise headless received events
are not generated unique events/s, and the 100-slot dense fixture does not
reproduce 150 events/s, the full tier mix, or the specified vehicle load.
These require authoritative candidate/tier and generated-event instrumentation
and controlled placement/event fixtures before the numerical bandwidth gates
can be counted as exact M2-020 acceptance. No-loss byte-for-byte regression,
HUD notices and CPU benchmark are outside this runner.

Tool-only checks (no build):

```sh
PYTHONDONTWRITEBYTECODE=1 M2_020_TEST_NETEM=1 python3 scripts/netem/m2-020/test_run.py -v
```

The optional privileged check uses actual TCP and UDP sockets (not Frontline)
to check direction filtering, unshaped metrics traffic and weak qdisc removal.
