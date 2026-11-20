"""Parser checks and optional live TCP/UDP filter checks (no Rust binaries or builds)."""

import os
import io
from contextlib import redirect_stderr
from pathlib import Path
import socket
import subprocess
import sys
import unittest

from run import PROFILES, bandwidth, command, netns, parse_args, population, qdisc, shape


class Summaries(unittest.TestCase):
    def test_cli_pick_defaults_and_reject_shared_weak_shaper(self):
        args = ["--server", sys.executable, "--headless", sys.executable, "--transport", "wt"]
        self.assertEqual(parse_args([*args, "--profile", "dense"]).seconds, 1800)
        self.assertEqual(parse_args([*args, "--profile", "typical"]).count, 20)
        self.assertEqual(parse_args([*args, "--profile", "middle"]).seconds, 10)
        self.assertEqual(PROFILES["middle"][3:], ("loss 2% delay 50ms", "loss 2% delay 50ms"))
        with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            parse_args([*args, "--profile", "weak", "--count", "20"])

    def test_session_metrics_are_required_on_both_teams(self):
        self.assertIsNone(population('server_players{team="us"} 4\n'))
        self.assertEqual(population('server_players{team="us"} 8\nserver_players{team="ger"} 12\n'), 20)

    def test_bandwidth_uses_window_delta_and_exact_threshold(self):
        def scrape(count, total, at_most_400):
            name = "net_bytes_out_per_client_kbps"
            return (f'{name}_count{{kind="wire"}} {count}\n'
                    f'{name}_sum{{kind="wire"}} {total}\n'
                    f'{name}_bucket{{kind="wire",le="400"}} {at_most_400}\n')

        before = scrape(50, 5000, 49)
        self.assertEqual(bandwidth(before, scrape(150, 10000, 148), "wire"), {
            "samples": 100, "mean_kbps": 50, "p99_at_most_400_kbps": True,
        })
        self.assertFalse(bandwidth(before, scrape(150, 10000, 147), "wire")["p99_at_most_400_kbps"])
        self.assertIsNone(bandwidth(before, "", "wire")["mean_kbps"])
        self.assertIsNone(bandwidth(before, scrape(1, 1, 1), "wire")["samples"])


class LiveFilters(unittest.TestCase):
    def test_filters_in_private_namespace(self):
        if not os.environ.get("M2_020_TEST_NETEM"):
            self.skipTest("set M2_020_TEST_NETEM=1 to run the privileged network check")
        for transport, profile in (("wt", "typical"), ("wt", "middle"), ("ws", "middle"), ("ws", "weak")):
            with self.subTest(transport=transport, profile=profile):
                subprocess.run(["unshare", "--net", "--", sys.executable,
                                str(Path(__file__).resolve()), netns(), transport, profile], check=True, timeout=15)


def probe(parent_ns, transport, profile):
    assert netns() != parent_ns
    command("ip", "link", "set", "lo", "up")
    kind = socket.SOCK_DGRAM if transport == "wt" else socket.SOCK_STREAM
    with socket.socket(socket.AF_INET, kind) as server, socket.socket(socket.AF_INET, kind) as client:
        server.bind(("127.0.0.1", 0))
        for sock in (server, client):
            sock.settimeout(3)
        if transport == "ws":
            server.listen()
        shape(server.getsockname()[1], transport, *PROFILES[profile][3:], 26)
        # An unrelated port, equivalent to the metrics listener, must not hit either qdisc.
        with socket.socket() as outside:
            outside.bind(("127.0.0.1", 0))
            outside.listen()
            with socket.create_connection(outside.getsockname(), timeout=1) as sock:
                with outside.accept()[0] as accepted:
                    sock.sendall(b"unshaped")
                    assert accepted.recv(32) == b"unshaped"
        assert all(d["packets"] == d["drops"] == 0 for d in qdisc() if d["kind"] == "netem")
        client.connect(server.getsockname())
        if transport == "wt":
            # Batches avoid assuming every datagram survives the seeded 5% qdisc.
            for _ in range(50):
                client.send(b"uplink")
            for _ in range(20):
                payload, sender = server.recvfrom(32)
                assert payload == b"uplink"
                server.sendto(b"downlink", sender)
            assert client.recv(32) == b"downlink"
        else:
            with server.accept()[0] as accepted:
                accepted.settimeout(3)
                client.sendall(b"uplink")
                assert accepted.recv(6, socket.MSG_WAITALL) == b"uplink"
                accepted.sendall(b"downlink")
                assert client.recv(8, socket.MSG_WAITALL) == b"downlink"
        leaves = {d["handle"]: d for d in qdisc() if d["kind"] == "netem"}
        assert set(leaves) == {"10:", "20:"}, leaves
        assert all(d["packets"] > 0 for d in leaves.values()), leaves
        if profile == "middle":
            for leaf in leaves.values():
                assert leaf["options"]["delay"]["delay"] == 0.05, leaf
                assert leaf["options"]["loss-random"]["loss"] == 0.02, leaf
        if profile == "weak":
            assert leaves["10:"]["options"]["rate"]["rate"] == 10000, leaves
            assert "rate" not in leaves["20:"]["options"], leaves
            command("tc", "qdisc", "del", "dev", "lo", "root")
            assert not any(d["kind"] == "netem" for d in qdisc())


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1].startswith("net:["):
        probe(*sys.argv[1:])
    else:
        unittest.main()
