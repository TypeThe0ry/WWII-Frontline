#!/usr/bin/env python3
"""Measure concurrent authenticated tickets on a local gateway and a fresh disk database."""

import argparse
import asyncio
from collections import Counter
from contextlib import suppress
from http.client import HTTPConnection, HTTPException
from ipaddress import IPv4Network
import json
import math
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time


CLIENT_NETWORK = IPv4Network("198.18.0.0/15")  # Benchmark-only addresses, sent as headers.
MATCH_CAP = 100
REQUESTS_PER_MATCH = MATCH_CAP * 2
STAGE_TIMEOUT_S = 15
TICKET_METRIC_STAGES = {
    "storage": ("pool_wait", "transaction"),
    "stage": ("auth", "request_parse", "eddsa_sign", "serialization", "log"),
}
TICKET_METRIC_SAMPLE = re.compile(
    r'gateway_ticket_(storage|stage)_duration_seconds_(sum|count)'
    r'\{stage="([^"]+)"\}\s+(\S+)(?:\s+\d+)?'
)


async def request(port, route, ip, payload, token=None):
    reader, writer = await asyncio.open_connection("127.0.0.1", port)
    try:
        return await post(reader, writer, route, ip, payload, token)
    finally:
        writer.close()
        with suppress(OSError):
            await writer.wait_closed()


async def post(reader, writer, route, ip, payload, token):
    body = json.dumps(payload).encode()
    headers = [f"POST {route} HTTP/1.1", "Host: localhost", "Connection: close",
               "Content-Type: application/json", f"Content-Length: {len(body)}",
               f"X-Forwarded-For: {ip}"]
    if token is not None:
        headers.append(f"Authorization: Bearer {token}")
    writer.write(("\r\n".join(headers) + "\r\n\r\n").encode() + body)
    await writer.drain()
    try:
        response_headers = await reader.readuntil(b"\r\n\r\n")
        status = int(response_headers.split()[1])
        response_body = await reader.read()
    except (EOFError, IndexError, ValueError) as exc:
        raise RuntimeError(f"{route} returned an incomplete or invalid HTTP response") from exc
    if status != 200:
        detail = response_body[:200].decode(errors="replace")
        raise RuntimeError(f"{route} returned HTTP {status}: {detail!r}")
    try:
        return json.loads(response_body)
    except ValueError as exc:
        raise RuntimeError(f"{route} returned invalid JSON") from exc


async def stage(name, work):
    try:
        return await asyncio.wait_for(work, STAGE_TIMEOUT_S)
    except asyncio.TimeoutError as exc:
        raise RuntimeError(f"{name} timed out after {STAGE_TIMEOUT_S}s") from exc
    except (RuntimeError, OSError, KeyError, TypeError) as exc:
        raise RuntimeError(f"{name}: {exc}") from exc


async def setup_clients(port, count):
    ips = [str(CLIENT_NETWORK.network_address + i + 1) for i in range(count)]
    replies = await asyncio.gather(*(request(port, "/v1/guest", ip, {}) for ip in ips),
                                   return_exceptions=True)
    clients = []
    for ip, reply in zip(ips, replies, strict=True):
        if isinstance(reply, BaseException):
            raise RuntimeError(f"client {ip}: {reply}") from reply
        clients.append((ip, reply["nick_suggestion"], reply["token"]))
    return clients


async def register_matches(port, protocol_version, internal, count):
    routes = []
    for index in range(count):
        match = await request(port, "/internal/matches/register", "127.0.0.1", {
            "name": "loadtest" if count == 1 else f"loadtest-{index + 1}",
            "map": "m0", "mode": "sandbox", "region": "local", "version": "test",
            "protocol_version": protocol_version, "data_hash": 1, "cap": MATCH_CAP,
            "addr": "127.0.0.1", "ws_port": 8081, "wt_port": 8082,
        }, internal)
        routes.append(f"/v1/servers/{match['match_id']}/ticket")
    return routes


async def issue_batch(port, clients, routes):
    writers = []
    go = asyncio.Event()

    async def connect():
        reader, writer = await asyncio.open_connection("127.0.0.1", port)
        writers.append(writer)
        return reader, writer

    async def connect_all():
        connections = await asyncio.gather(*(connect() for _ in clients), return_exceptions=True)
        for conn in connections:
            if isinstance(conn, BaseException):
                raise RuntimeError(str(conn)) from conn
        return connections

    async def issue(conn, route, client):
        ip, nick, token = client
        await go.wait()
        reply = await post(*conn, route, ip, {"nick": nick}, token)
        ticket = reply.get("ticket") if isinstance(reply, dict) else None
        if not isinstance(ticket, str) or not ticket:
            raise RuntimeError(f"{route} returned an empty or invalid ticket")
        return (time.perf_counter() - start) * 1000, ticket

    tasks = []
    try:
        connections = await stage("preconnect ticket clients", connect_all())
        tasks = [asyncio.create_task(issue(conn, routes[i % len(routes)], client))
                 for i, (conn, client) in enumerate(zip(connections, clients, strict=True))]
        # Start only after all sockets are connected; all completions use this origin.
        start = time.perf_counter()
        go.set()
        _, pending = await asyncio.wait(tasks, timeout=STAGE_TIMEOUT_S)
        for task in pending:
            task.cancel()
        results = await asyncio.gather(*tasks, return_exceptions=True)
        successes, errors = [], Counter()
        for result in results:
            if isinstance(result, asyncio.CancelledError):
                errors[f"ticket response timed out after {STAGE_TIMEOUT_S}s"] += 1
            elif isinstance(result, Exception):
                errors[f"{type(result).__name__}: {result}"] += 1
            else:
                successes.append(result)
        return successes, errors
    finally:
        for task in tasks:
            if not task.done():
                task.cancel()
        for writer in writers:
            writer.close()
        await asyncio.gather(*tasks, *(w.wait_closed() for w in writers), return_exceptions=True)


async def wait_ready(process, port):
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError(f"gateway exited during startup (status {process.returncode}); "
                               "check gateway stderr")
        try:
            _, writer = await asyncio.open_connection("127.0.0.1", port)
        except ConnectionRefusedError:
            await asyncio.sleep(0.05)
        else:
            writer.close()
            await writer.wait_closed()
            return
    raise RuntimeError(f"gateway did not listen on 127.0.0.1:{port} within 5s; check gateway stderr")


def stop_gateway(process):
    # Signal only the gateway child started by this run, then reap it before removing its DB.
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def scrape_ticket_metrics(port):
    conn = HTTPConnection("127.0.0.1", port, timeout=5)
    try:
        conn.request("GET", "/metrics")
        response = conn.getresponse()
        if response.status != 200:
            raise RuntimeError(f"GET /metrics returned HTTP {response.status}")
        text = response.read().decode()
    except (OSError, HTTPException, UnicodeError) as exc:
        raise RuntimeError(f"GET /metrics: {exc}") from exc
    finally:
        conn.close()

    samples = {}
    for line in text.splitlines():
        match = TICKET_METRIC_SAMPLE.fullmatch(line)
        if match:
            family, kind, name, value = match.groups()
            samples[family, name, kind] = value

    metrics = {}
    for family, names in TICKET_METRIC_STAGES.items():
        stages = metrics[f"ticket_{family}_metrics"] = {}
        for name in names:
            try:
                seconds = float(samples[family, name, "sum"])
                count = float(samples[family, name, "count"])
            except (KeyError, ValueError) as exc:
                raise RuntimeError(f"/metrics is missing a valid ticket {family}/{name} sum/count; "
                                   "use an instrumented gateway or omit --metrics") from exc
            if not math.isfinite(seconds) or seconds < 0 or not count.is_integer() or count < 0:
                raise RuntimeError(f"/metrics returned an invalid ticket {family}/{name} sum/count")
            stages[name] = {"count": int(count), "mean_ms": seconds / count * 1000 if count else None}
    return metrics


async def run(binary, protocol_version, count, run_number, p95_limit, metrics_enabled):
    metrics = {}
    metrics_error = None
    with tempfile.TemporaryDirectory(prefix="gateway-load-") as tmp:
        internal = "local-load-fixture"
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        process = subprocess.Popen(
            [binary], cwd=tmp,
            env={**os.environ, "GATEWAY_BIND": f"127.0.0.1:{port}",
                 "TICKET_SIGNING_KEY_FILE": str(Path(tmp) / "ticket.key"),
                 "TRUST_FORWARDED": "1", "DATABASE_URL": "sqlite://gateway.db",
                 "INTERNAL_API_TOKEN": internal, "TOKEN_SECRET": "local-load-token-fixture"},
            stdout=subprocess.DEVNULL,
        )
        try:
            await wait_ready(process, port)
            clients = await stage("guest login", setup_clients(port, count))
            routes = await stage("match registration", register_matches(
                port, protocol_version, internal, math.ceil(count / REQUESTS_PER_MATCH)))
            results, errors = await issue_batch(port, clients, routes)
            if metrics_enabled:
                try:
                    metrics = await stage("scrape ticket metrics",
                                          asyncio.to_thread(scrape_ticket_metrics, port))
                except RuntimeError as exc:
                    metrics_error = f"gateway PID {process.pid}: {exc}"
                    metrics = {"metrics_error": metrics_error}
        except (RuntimeError, OSError) as exc:
            raise RuntimeError(f"gateway PID {process.pid}: {exc}") from exc
        finally:
            stop_gateway(process)

    durations = sorted(ms for ms, _ in results)
    unique = len({ticket for _, ticket in results})

    def percentile(q):
        return durations[math.ceil(q * len(durations)) - 1] if durations else None

    def rounded(ms):
        return round(ms, 3) if ms is not None else None

    p95 = percentile(0.95)
    print(json.dumps({"run": run_number, "gateway_pid": process.pid,
                      "matches": len(routes), "requests": count,
                      "successful_requests": len(results), "failed_requests": count - len(results),
                      "unique_tickets": unique, "duplicate_tickets": len(results) - unique,
                      "p50_ms": rounded(percentile(0.50)), "p95_ms": rounded(p95),
                      "p99_ms": rounded(percentile(0.99)), "max_ms": rounded(percentile(1)),
                      **metrics}),
          flush=True)
    for error, n in errors.most_common(5):
        print(f"Run {run_number}: {n} ticket request(s) failed: {error}", file=sys.stderr)
    if len(errors) > 5:
        print(f"Run {run_number}: {len(errors) - 5} more distinct errors", file=sys.stderr)
    if unique != len(results):
        print(f"Run {run_number}: got {unique} unique tickets from {len(results)} successful "
              "requests", file=sys.stderr)
    if metrics_error:
        print(f"Run {run_number}: {metrics_error}", file=sys.stderr)
    too_slow = p95_limit is not None and p95 is not None and p95 > p95_limit
    if too_slow:
        print(f"Run {run_number}: p95 {p95:.3f} ms exceeds {p95_limit:g} ms; use "
              "--report-only to measure without the latency gate", file=sys.stderr)
    return unique == count and not too_slow and metrics_error is None


async def main(args):
    loop = asyncio.get_running_loop()
    main_task = asyncio.current_task()
    interrupted_by = None

    def interrupt(signum, _frame):
        nonlocal interrupted_by
        if interrupted_by is None:
            interrupted_by = signum
            loop.call_soon_threadsafe(main_task.cancel)

    handlers = {sig: signal.signal(sig, interrupt) for sig in (signal.SIGINT, signal.SIGTERM)}
    passed = True
    try:
        for run_number in range(1, args.runs + 1):
            try:
                if not await run(args.binary, args.protocol_version, args.concurrency,
                                 run_number, args.p95_ms, args.metrics):
                    passed = False
            except (RuntimeError, OSError) as exc:
                print(f"Run {run_number}: {exc}", file=sys.stderr)
                return 1
    except asyncio.CancelledError:
        if interrupted_by is None:
            raise
        print("Interrupted; gateway child cleaned up", file=sys.stderr)
        return 128 + interrupted_by
    finally:
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    return 0 if passed else 1


def positive_int(value):
    n = int(value)
    if n <= 0:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return n


def positive_ms(value):
    ms = float(value)
    if not math.isfinite(ms) or ms <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number")
    return ms


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=lambda p: Path(p).resolve())
    parser.add_argument("--protocol-version", type=int, required=True)
    parser.add_argument("--concurrency", type=positive_int, default=200,
                        help="ticket requests per batch (default: 200)")
    parser.add_argument("--runs", type=positive_int, default=1,
                        help="independent runs with a fresh gateway and DB each; one JSON line per batch")
    parser.add_argument("--metrics", action="store_true",
                        help="GET /metrics after each batch and report ticket histogram means/counts")
    gate = parser.add_mutually_exclusive_group()
    gate.add_argument("--p95-ms", type=positive_ms, default=50.0,
                      help="maximum p95 in ms for each batch (default: 50)")
    gate.add_argument("--report-only", action="store_const", const=None, dest="p95_ms",
                      help="disable the latency gate; still require all tickets to be unique")
    args = parser.parse_args()
    if not 0 <= args.protocol_version <= 65535:
        parser.error("--protocol-version must fit in an unsigned 16-bit integer")
    if args.concurrency > CLIENT_NETWORK.num_addresses - 2:
        parser.error(f"--concurrency exceeds the {CLIENT_NETWORK.num_addresses - 2} fixture IPs")
    try:
        raise SystemExit(asyncio.run(main(args)))
    except KeyboardInterrupt:
        raise SystemExit(130) from None
