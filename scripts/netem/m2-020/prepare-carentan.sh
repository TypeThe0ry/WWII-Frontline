#!/usr/bin/env bash
# Bake disposable Carentan M1 map data without changing the worktree or building Rust.
set -euo pipefail
cd "$(dirname "$0")/../../.."
out=$(mktemp -d /data/scratch/m2-020-carentan-XXXXXX)
trap 'rm -rf -- "$out"' EXIT
mkdir -p "$out/data/maps" "$out/reports"
cp -a data/maps/carentan "$out/data/maps/"
map="$out/data/maps/carentan"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools/map-pipeline python3 -m map_pipeline chunks-m1 \
  --repo "$out" --out "$map" --dest "$map/m1/chunks" --workers 6 >&2
python3 - "$map/m1" <<'PY'
import hashlib
import json
from pathlib import Path
import sys
import zlib

root = Path(sys.argv[1])
chunks = sorted((root / "chunks").glob("c_*.fcol"))
if not chunks:
    raise SystemExit("no FCOL output")
manifest = []
for packed in chunks:
    data = zlib.decompress(packed.read_bytes())
    if data[:4] != b"FCOL" or len(data) != 16 + 65 * 65 * 2 + 64 * 64:
        raise ValueError(f"unexpected FCOL: {packed}")
    raw = packed.with_suffix(".bin")
    raw.write_bytes(data)
    manifest.append(f"{hashlib.sha256(data).hexdigest()}  {raw.relative_to(root)}\n")
(root / "fixture-chunks.sha256").write_text("".join(manifest))

# Same synthetic patrol points as the preceding Carentan netem soak.
spawns_file = root / "spawns.json"
spawns = json.loads(spawns_file.read_text())
points = [p for line in json.loads((root / "zones.json").read_text())["lines"] for p in line["points"]]
patrols = [{"pos": p["pos"]} for p in points if p["id"] in {"L3a", "L3b"}]
if len(patrols) != 2:
    raise ValueError("missing L3a/L3b fixture patrol points")
spawns["patrol_points"] = patrols
spawns_file.write_text(json.dumps(spawns, indent=2) + "\n")
print(f"Prepared {len(chunks)} FCOL chunks, 2 synthetic patrol points", file=sys.stderr)
PY
printf '%s\n' "$out/data"
trap - EXIT
