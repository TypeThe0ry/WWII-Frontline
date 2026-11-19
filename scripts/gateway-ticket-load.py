#!/usr/bin/env python3
"""Measure 200 concurrent authenticated tickets on a local gateway and a fresh disk database."""

import argparse
import asyncio
import json
import math
import os
from pathlib import Path
import socket
import tempfile
import time


async def request(port, route, ip, payload, token=None):
    reader, writer = await asyncio.open_connection("127.0.0.1", port)
    try:
        return await post(reader, writer, route, ip, payload, token)
    finally:
        writer.close()
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
    response_headers = await reader.readuntil(b"\r\n\r\n")
    status = int(response_headers.split()[1])
    if status != 200:
        raise RuntimeError(f"{route} returned HTTP {status}")
    return json.loads(await reader.read())


async def run(binary, protocol_version):
    count = 200
    with tempfile.TemporaryDirectory(prefix="gateway-load-") as tmp:
        internal = "local-load-fixture"
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        process = await asyncio.create_subprocess_exec(
            binary, cwd=tmp,
            env={**os.environ, "GATEWAY_BIND": f"127.0.0.1:{port}",
                 "TICKET_SIGNING_KEY_FILE": str(Path(tmp) / "ticket.key"),
                 "TRUST_FORWARDED": "1", "DATABASE_URL": "sqlite://gateway.db",
                 "INTERNAL_API_TOKEN": internal, "TOKEN_SECRET": "local-load-token-fixture"},
            stdout=asyncio.subprocess.DEVNULL,
        )
        try:
            for _ in range(100):
                try:
                    reader, writer = await asyncio.open_connection("127.0.0.1", port)
                    writer.close()
                    await writer.wait_closed()
                    break
                except ConnectionRefusedError:
                    await asyncio.sleep(0.05)
            else:
                raise RuntimeError("Gateway did not start")
            ips = [f"192.0.2.{i + 1}" for i in range(count)]
            guests = await asyncio.gather(*(request(port, "/v1/guest", ip, {}) for ip in ips))
            match = await request(port, "/internal/matches/register", "127.0.0.1", {
                "name": "loadtest", "map": "m0", "mode": "sandbox", "region": "local",
                "version": "test", "protocol_version": protocol_version, "data_hash": 1,
                "cap": 100, "addr": "127.0.0.1", "ws_port": 8081, "wt_port": 8082,
            }, internal)
            route = f"/v1/servers/{match['match_id']}/ticket"
            connections = await asyncio.gather(*(asyncio.open_connection("127.0.0.1", port)
                                                   for _ in ips))
            start = time.perf_counter()

            async def issue(conn, ip, guest):
                reader, writer = conn
                try:
                    ticket = await post(reader, writer, route, ip, {"nick": guest["nick_suggestion"]},
                                        guest["token"])
                    return (time.perf_counter() - start) * 1000, ticket["ticket"]
                finally:
                    writer.close()
                    await writer.wait_closed()

            results = await asyncio.wait_for(asyncio.gather(*(issue(conn, ip, guest)
                    for conn, ip, guest in zip(connections, ips, guests, strict=True))), 15)
            durations = sorted(ms for ms, _ in results)
            unique = len({ticket for _, ticket in results})
            p95 = durations[math.ceil(0.95 * count) - 1]
            print(json.dumps({"requests": len(results), "unique_tickets": unique,
                              "p50_ms": round(durations[count // 2 - 1], 3),
                              "p95_ms": round(p95, 3), "max_ms": round(durations[-1], 3)}))
            return 0 if unique == count and p95 <= 50 else 1
        finally:
            if process.returncode is None:
                process.terminate()
            await process.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=lambda p: Path(p).resolve())
    parser.add_argument("--protocol-version", type=int, required=True)
    args = parser.parse_args()
    raise SystemExit(asyncio.run(asyncio.wait_for(run(args.binary, args.protocol_version), 30)))
