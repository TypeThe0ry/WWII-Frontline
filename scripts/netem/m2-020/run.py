#!/usr/bin/env python3
"""Run prebuilt milestone server/headless binaries in a disposable network namespace."""

import argparse
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import tempfile
import time
from urllib.request import ProxyHandler, build_opener


ROOT = Path(__file__).resolve().parents[3]
MATCH_ID = "0a9003ec-f0b5-4ac1-a09a-81d05072d401"
PROFILES = {
    # count, bots, measurement seconds, one-way downstream and upstream netem
    "typical": (20, 40, 1800, "loss 5% delay 40ms 20ms", "loss 5% delay 40ms 20ms"),
    "middle": (20, 40, 10, "loss 2% delay 50ms", "loss 2% delay 50ms"),
    "dense": (20, 80, 1800, "", ""),
    "weak": (1, 12, 60, "loss 5% delay 75ms rate 80kbit", "loss 5% delay 75ms"),
    "clear": (20, 40, 1800, "", ""),
}
STARTUP_SLACK = 30
HTTP = build_opener(ProxyHandler({}))


def write_json(path, obj):
    path.write_text(json.dumps(obj, indent=2) + "\n")


def command(*args):
    return subprocess.check_output(args, text=True)


def netns():
    return os.readlink("/proc/self/ns/net")


def qdisc():
    return json.loads(command("tc", "-s", "-j", "qdisc", "show", "dev", "lo"))


def shape(port, transport, down, up, seed):
    # One ordered independent qdisc per direction. Unmatched traffic (including /metrics)
    # goes to the unshaped third band. 80 kbit/s is per client only for weak's single client.
    command("tc", "qdisc", "add", "dev", "lo", "root", "handle", "1:", "prio",
            "bands", "3", "priomap", *(["2"] * 16))
    proto = "udp" if transport == "wt" else "tcp"
    for i, (args, field) in enumerate(((down, "src_port"), (up, "dst_port")), start=1):
        command("tc", "qdisc", "add", "dev", "lo", "parent", f"1:{i}", "handle", f"{i}0:",
                "netem", *args.split(), "seed", str(seed + i))
        command("tc", "filter", "add", "dev", "lo", "parent", "1:", "protocol", "ip", "pref",
                str(i), "flower", "ip_proto", proto, field, str(port), "classid", f"1:{i}")


def launch(stack, role, args, cpus, run, env):
    if cpus:
        args = ["taskset", "-c", cpus, *args]
    log = stack.enter_context((run / f"{role}.log").open("w"))
    proc = subprocess.Popen(args, cwd=run, env=env, stdout=log, stderr=subprocess.STDOUT)
    stack.callback(stop, proc)
    with (run / "pids.tsv").open("a") as pids:
        print(role, proc.pid, sep="\t", file=pids)
    return proc


def stop(proc):
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()


def listeners(server, log):
    found = {}
    deadline = time.monotonic() + 30
    with log.open() as stream:
        while time.monotonic() < deadline:
            while True:
                offset = stream.tell()
                line = stream.readline()
                if not line.endswith("\n"):
                    # The writer may not have completed this record yet.
                    stream.seek(offset)
                    break
                try:
                    entry = json.loads(line)
                except ValueError:
                    continue
                fields = entry.get("fields", entry)
                if fields.get("event") in {"startup", "map_loaded", "ws_listening",
                                            "wt_listening", "metrics_listening"}:
                    found[fields["event"]] = fields
            if {"ws_listening", "wt_listening", "metrics_listening"} <= found.keys():
                return found
            if server.poll() is not None:
                break
            time.sleep(0.1)
    raise RuntimeError(f"server did not publish listeners; inspect {log}")


def metric(text, name, labels=""):
    # Parse only the scalar samples needed for this report; retain the full scrapes.
    m = re.search(rf"^{re.escape(name + labels)} ([^\s]+)", text, re.MULTILINE)
    return float(m.group(1)) if m else None


def population(text):
    teams = [metric(text, "server_players", f'{{team="{team}"}}') for team in ("us", "ger")]
    return sum(teams) if None not in teams else None


def delta(before, after, name, labels=""):
    a, b = metric(before, name, labels), metric(after, name, labels)
    return b - a if a is not None and b is not None and b >= a else None


def bandwidth(before, after, kind):
    name = "net_bytes_out_per_client_kbps"
    label = f'{{kind="{kind}"}}'
    count = delta(before, after, name + "_count", label)
    total = delta(before, after, name + "_sum", label)
    # Exact threshold bucket: no assumption about distribution inside other buckets.
    bucket = delta(before, after, name + "_bucket", f'{{kind="{kind}",le="400"}}')
    return {
        "samples": count,
        "mean_kbps": total / count if total is not None and count else None,
        "p99_at_most_400_kbps": bucket >= 0.99 * count if bucket is not None and count else None,
    }


def startup_error(phase, server, client, run):
    if server.poll() is not None:
        return f"server exited ({server.returncode}) during {phase}; inspect {run / 'server.log'}"
    if client.poll() is not None:
        return f"headless exited ({client.returncode}) during {phase}; inspect {run / 'headless.log'}"
    return None


def run_case(a, run, down, up):
    command("ip", "link", "set", "lo", "up")
    env = {k: v for k, v in os.environ.items() if not k.startswith("FRONTLINE_")
           and k not in {"GATEWAY_URL", "INTERNAL_API_TOKEN", "TICKET_PUBLIC_KEY_FILE", "FL_AOI_TRACE"}}
    env["TMPDIR"] = str(run)
    if a.sim_cpu is not None:
        env["FRONTLINE_SIM_CPU"] = str(a.sim_cpu)

    mode = "assault" if a.map == "carentan/m1" else "sandbox"
    server_args = [a.server, "--ws-port", "0", "--wt-port", "0", "--metrics-port", "0",
                   "--log-level", "info", "--dev-cert", "--bots", str(a.bots), "--match-id", MATCH_ID,
                   "--map", a.map, "--mode", mode, "--data-dir", a.data, "--report", run / "server.json"]
    with ExitStack() as stack:
        server = launch(stack, "server", server_args, a.server_cpus, run, env)
        ready = listeners(server, run / "server.log")
        port = ready[a.transport + "_listening"]["port"]
        metrics_port = ready["metrics_listening"]["port"]
        recovery = a.recovery if a.profile == "weak" else 0
        duration = a.warmup + a.seconds + STARTUP_SLACK + recovery
        client_args = [a.headless, "--url", f"127.0.0.1:{port}", "--transport", a.transport,
                       "--count", str(a.count), "--duration", str(duration), "--warmup", str(a.warmup),
                       "--seed", str(a.seed), "--map", a.map, "--data-dir", a.data,
                       "--report", run / "headless.json"]
        if mode == "assault":
            client_args += ["--deploy-hq"]
        if a.transport == "wt":
            client_args += ["--cert-hash", ready["wt_listening"]["cert_sha256_hex"]]
        if down:
            shape(port, a.transport, down, up, a.seed)
            write_json(run / "filters.json", json.loads(command(
                "tc", "-j", "filter", "show", "dev", "lo", "parent", "1:")))
        write_json(run / "qdisc-start.json", qdisc())
        stack.callback(save_exit_qdisc, run)
        write_json(run / "run.json", {
            "environment": "PI-109 shared workstation / not reference hardware",
            "host": platform.node(), "kernel": platform.release(), "match_id": MATCH_ID,
            "profile": a.profile, "requested_window_s": a.seconds, "netns": netns(),
            "seed": a.seed, "down_netem": down, "up_netem": up,
            "server_startup": ready.get("startup"), "map_loaded": ready.get("map_loaded"),
            "binaries_sha256": {str(p): sha256(p) for p in (a.server, a.headless)},
            "server_argv": list(map(str, server_args)), "client_argv": list(map(str, client_args)),
            "cpu_affinity_requested": [a.server_cpus, a.client_cpus, a.sim_cpu],
        })
        client = launch(stack, "headless", client_args, a.client_cpus, run, env)
        launched = time.monotonic()
        samples = []

        def scrape(phase):
            error = startup_error(phase, server, client, run)
            if error:
                raise RuntimeError(error)
            with HTTP.open(f"http://127.0.0.1:{metrics_port}/metrics", timeout=3) as response:
                text = response.read().decode()
            now = time.monotonic()
            path = f"metrics-{len(samples):04d}.txt"
            (run / path).write_text(text)
            sample = {"phase": phase, "elapsed_s": now - launched, "file": path,
                      "sessions": population(text), "bots": metric(text, "server_bots")}
            samples.append(sample)
            with (run / "samples.jsonl").open("a") as timeline:
                print(json.dumps(sample), file=timeline)
            return now, text

        def window(start, seconds, phase, require_clients):
            deadline = start + seconds
            while True:
                time.sleep(max(0, min(a.interval, deadline - time.monotonic())))
                end, text = scrape(phase)
                if require_clients and samples[-1]["sessions"] != a.count:
                    raise RuntimeError(f"expected {a.count} sessions, saw {samples[-1]['sessions']} during {phase}")
                if end >= deadline:
                    return end, text

        # Allow admission and HQ deployment before the fixed wall-clock measurement window.
        # HQ success is checked again from the completed client reports.
        while True:
            now, _ = scrape("admission")
            if samples[-1]["sessions"] == a.count:
                break
            if now - launched >= STARTUP_SLACK / 2:
                raise RuntimeError(f"only {samples[-1]['sessions']}/{a.count} sessions joined; inspect logs")
            time.sleep(0.25)
        start, before = window(now, a.warmup, "warmup", True)
        end, after = window(start, a.seconds, "measure", True)
        write_json(run / "qdisc-end.json", qdisc())
        if recovery:
            command("tc", "qdisc", "del", "dev", "lo", "root")
            restored = time.monotonic()
            write_json(run / "restored.json", {"elapsed_s": restored - launched, "qdisc": qdisc()})
            window(restored, recovery, "recovery", True)

        # Keep the official report (including late disconnects) rather than stopping clients early.
        status = client.wait(timeout=max(5, launched + duration + STARTUP_SLACK - time.monotonic()))
        if server.poll() is not None:
            raise RuntimeError(f"server exited ({server.returncode}) before the client report completed")
        report = json.loads((run / "headless.json").read_text())
        clients = report["per_client"]
        window_s = end - start
        valid = status == 0 and report["transport"] == a.transport and len(clients) == a.count and all(
            c["player_id"] is not None and not c["join_error"] and not c["disconnects"]
            and c["snapshots"] > 0 and c["connected_s"] >= window_s
            and (mode != "assault" or c.get("hq_spawn_tick") is not None) for c in clients)
        starved = delta(before, after, "aoi_starved_total")
        write_json(run / "summary.json", {
            "classification": "fixture only; not M2-020 acceptance",
            "observed_window_s": window_s, "valid_client_run": valid,
            "server_downlink": {kind: bandwidth(before, after, kind) for kind in ("payload", "wire")},
            "aoi_starved_per_min": None if starved is None else starved * 60 / window_s,
            "minimum_sampled_sessions": min(s["sessions"] for s in samples if s["phase"] == "measure"),
            "sampled_bot_population": sorted({s["bots"] for s in samples if s["phase"] == "measure"
                                              and s["bots"] is not None}),
            "headless_received_events_total": sum(c["events"] for c in clients),
            "visible_aois_and_tiers_verified": False, "generated_event_rate_verified": False,
            "t3_spacing_and_regulator_behavior_verified": False,
        })
        if not valid:
            raise RuntimeError(f"invalid headless result (exit {status}); inspect {run / 'summary.json'}")


def save_exit_qdisc(run):
    try:
        write_json(run / "qdisc-exit.json", qdisc())
    except (OSError, subprocess.SubprocessError) as exc:
        print(f"m2-020: cannot save exit qdisc: {exc}", file=sys.stderr)


def sha256(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def parse_args(argv):
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("server", "headless"):
        p.add_argument(f"--{name}", type=Path, required=True, help="path to a prebuilt binary")
    p.add_argument("--transport", choices=("wt", "ws"), required=True)
    p.add_argument("--profile", choices=PROFILES, required=True)
    p.add_argument("--data", type=Path, default=ROOT / "data", help="prepared game data directory")
    p.add_argument("--map", choices=("m0", "carentan/m1"), default="carentan/m1")
    for field in ("count", "bots", "seconds", "sim-cpu"):
        p.add_argument(f"--{field}", type=int)
    p.add_argument("--seed", type=int, default=26)
    p.add_argument("--warmup", type=int, default=10)
    p.add_argument("--recovery", type=int, default=20, help="post-netem seconds for weak only")
    p.add_argument("--interval", type=float, help="scrape interval, default 10 s (0.5 s for weak)")
    p.add_argument("--server-cpus")
    p.add_argument("--client-cpus")
    a = p.parse_args(argv)
    for field, default in zip(("count", "bots", "seconds"), PROFILES[a.profile]):
        if getattr(a, field) is None:
            setattr(a, field, default)
    if a.interval is None:
        a.interval = 0.5 if a.profile == "weak" else 10
    if not (1 <= a.count <= 120 and 0 <= a.bots <= 128 - a.count and a.seconds > 0
            and a.warmup >= 0 and a.recovery >= 10 and 0.1 <= a.interval <= 60
            and 0 <= a.seed < 2**64 - 2):
        p.error("invalid count, bot reservation, duration, interval or seed")
    if a.profile == "weak" and (a.count != 1 or a.interval > 1):
        p.error("weak requires one client per downstream 80 kbit/s shaper and <=1 s scrape interval")
    for name in ("server", "headless", "data"):
        setattr(a, name, getattr(a, name).resolve())
    for binary in (a.server, a.headless):
        if not binary.is_file() or not os.access(binary, os.X_OK):
            p.error(f"missing executable: {binary}; this tool never builds binaries")
    return a


def main():
    a = parse_args(sys.argv[1:])
    original = os.environ.pop("M2_020_PARENT_NETNS", None)
    if original is None:
        os.environ["M2_020_PARENT_NETNS"] = netns()
        os.execvp("unshare", ["unshare", "--net", "--", sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]])
    if original == netns():
        raise RuntimeError("refusing to change qdiscs in the parent network namespace")
    run = Path(tempfile.mkdtemp(prefix=f"m2-020-{a.profile}-{a.transport}-", dir="/data/scratch"))
    print(f"M2-020 fixture artifacts: {run}", flush=True)

    def interrupted(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, interrupted)
    run_case(a, run, *PROFILES[a.profile][3:])


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        sys.exit(f"m2-020: {error}")
