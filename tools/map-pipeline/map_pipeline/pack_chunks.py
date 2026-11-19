"""M1-035 `.fpak` chunk packing & M1-005 manifest fragment generation (pure Python, byte-identical
to `tools/asset-pipeline/src/fpak/writer.ts` FPAK v1 container specification)."""
from __future__ import annotations

import hashlib
import json
import re
import struct
import zlib
from pathlib import Path

FPAK_MAGIC = b"FPAK"
FPAK_MAGIC_END = b"KAPF"
FPAK_VERSION = 1
HEADER_BYTES = 32
FOOTER_BYTES = 32
ALIGN = 16
INDEX_FIXED_BYTES = 26  # <HBBHQI8s
KIND_META = 3
CODEC_RAW = 0
LOGICAL_ID_RE = re.compile(r"^[a-z][a-z0-9_]*(@[a-z0-9_]+)?$")
STANDALONE_EXT = {".glb", ".gltf", ".ktx2", ".ogg", ".opus", ".wav", ".png", ".jpg", ".jpeg"}
DEFAULT_MAX_PACK_BYTES = 16 * 1024 * 1024
MAP_CARENTAN_MEDIUM_BUDGET = 157_286_400  # 150 MiB (ARCHITECTURE §11.4)
DETERMINISTIC_BUILD_TS = "1970-01-01T00:00:00.000Z"


def align_up(n: int, a: int = ALIGN) -> int:
    return ((n + a - 1) // a) * a


def publish_path(sha256_hex: str) -> str:
    return f"/a/{sha256_hex[:2]}/{sha256_hex[2:16]}.fpak"


def write_fpak(entries: list[tuple[str, int, int, bytes]]) -> tuple[bytes, list[dict], str]:
    """Encode `(logical_id, kind, codec, payload)` tuples into an FPAK v1 blob.
    Returns `(blob, index_rows, sha256_hex)`."""
    seen: set[str] = set()
    index_rows: list[dict] = []
    off = HEADER_BYTES
    for logical_id, kind, codec, payload in entries:
        if not LOGICAL_ID_RE.match(logical_id) or len(logical_id.encode("utf-8")) > 255:
            raise ValueError(f"invalid FPAK logical id: {logical_id!r}")
        if logical_id in seen:
            raise ValueError(f"duplicate FPAK logical id: {logical_id!r}")
        seen.add(logical_id)
        sha_full = hashlib.sha256(payload).digest()
        index_rows.append({
            "logical_id": logical_id,
            "kind": kind,
            "codec": codec,
            "offset": off,
            "bytes": len(payload),
            "sha8": sha_full[:8],
            "sha256": sha_full.hex(),
            "index_bytes": INDEX_FIXED_BYTES + len(logical_id.encode("utf-8")),
        })
        off += align_up(len(payload), ALIGN)

    index_offset = off
    index_buf = bytearray()
    for row in index_rows:
        id_b = row["logical_id"].encode("utf-8")
        index_buf += struct.pack(
            "<HBBHQI8s",
            len(id_b),
            row["kind"],
            0,
            row["codec"],
            row["offset"],
            row["bytes"],
            row["sha8"],
        )
        index_buf += id_b
    index_bytes = len(index_buf)
    total = index_offset + index_bytes + FOOTER_BYTES
    out = bytearray(total)
    struct.pack_into("<4sHHIIQ", out, 0, FPAK_MAGIC, FPAK_VERSION, 0, len(index_rows), 0, index_offset)
    for row, (_, _, _, payload) in zip(index_rows, entries):
        out[row["offset"] : row["offset"] + len(payload)] = payload
    out[index_offset : index_offset + index_bytes] = index_buf
    crc = zlib.crc32(index_buf) & 0xFFFFFFFF
    f_off = index_offset + index_bytes
    struct.pack_into("<QIIIH", out, f_off, index_offset, index_bytes, len(index_rows), crc, FPAK_VERSION)
    out[f_off + 28 : f_off + 32] = FPAK_MAGIC_END
    blob = bytes(out)
    return blob, index_rows, hashlib.sha256(blob).hexdigest()


def verify_fpak(blob: bytes) -> list[dict]:
    """Read and verify an FPAK v1 blob (footer magic, index CRC32, per-entry sha8)."""
    if len(blob) < HEADER_BYTES + FOOTER_BYTES:
        raise ValueError("fpak too small")
    if blob[:4] != FPAK_MAGIC or blob[-4:] != FPAK_MAGIC_END:
        raise ValueError("fpak magic mismatch")
    f_off = len(blob) - FOOTER_BYTES
    index_offset, index_bytes, count, crc, ver = struct.unpack_from("<QIIIH", blob, f_off)
    if ver != FPAK_VERSION:
        raise ValueError(f"unsupported fpak version {ver}")
    if index_offset + index_bytes + FOOTER_BYTES != len(blob):
        raise ValueError("fpak footer layout mismatch")
    idx_raw = blob[index_offset : index_offset + index_bytes]
    if (zlib.crc32(idx_raw) & 0xFFFFFFFF) != crc:
        raise ValueError("fpak index crc32 mismatch")
    pos = 0
    rows = []
    for _ in range(count):
        id_len, kind, _r, codec, off, nbytes, sha8 = struct.unpack_from("<HBBHQI8s", idx_raw, pos)
        pos += INDEX_FIXED_BYTES
        logical_id = idx_raw[pos : pos + id_len].decode("utf-8")
        pos += id_len
        payload = blob[off : off + nbytes]
        if hashlib.sha256(payload).digest()[:8] != sha8:
            raise ValueError(f"{logical_id}: payload sha8 mismatch")
        rows.append({"logical_id": logical_id, "kind": kind, "codec": codec, "offset": off, "bytes": nbytes, "sha256": hashlib.sha256(payload).hexdigest()})
    if pos != index_bytes:
        raise ValueError("fpak index trailing bytes")
    return rows


def find_standalone(root: Path) -> list[str]:
    if not root.is_dir():
        return []
    return sorted(
        str(p.relative_to(root))
        for p in root.rglob("*")
        if p.is_file() and p.suffix.lower() in STANDALONE_EXT
    )


def pack_chunks(
    chunks_dir: Path,
    fpak_out: Path,
    manifest_out: Path,
    map_id: str = "carentan",
    group_size: int = 4,
    max_pack_bytes: int = DEFAULT_MAX_PACK_BYTES,
) -> dict:
    """Pack `b_{bx}_{bz}.fchk` + `c_{bx}_{bz}.fcol` into 4x4 spatial `.fpak` bundles under `fpak_out`
    and write an M1-005 compatible manifest fragment to `manifest_out`."""
    fchks = sorted(chunks_dir.glob("b_*_*.fchk"))
    if not fchks:
        raise FileNotFoundError(f"no b_*_*.fchk files found in {chunks_dir}")

    # Parse (bx, bz) and group by 4x4 region
    groups: dict[tuple[int, int], list[tuple[int, int, Path, Path]]] = {}
    for p in fchks:
        parts = p.stem.split("_")
        bx, bz = int(parts[1]), int(parts[2])
        cpath = chunks_dir / f"c_{bx}_{bz}.fcol"
        if not cpath.is_file():
            raise FileNotFoundError(f"missing collision chunk {cpath}")
        gk = (bx // group_size, bz // group_size)
        groups.setdefault(gk, []).append((bx, bz, p, cpath))

    # Clean stale .fpak files in fpak_out/a/ so reruns are strictly deterministic
    a_dir = fpak_out / "a"
    if a_dir.is_dir():
        for old in a_dir.rglob("*.fpak"):
            old.unlink()

    assets: dict[str, dict] = {}
    packs_meta: list[dict] = []

    def flush(group_name: str, batch: list[tuple[str, int, int, bytes]]) -> None:
        if not batch:
            return
        blob, rows, sha = write_fpak(batch)
        pub = publish_path(sha)
        dst = fpak_out / pub.lstrip("/")
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(blob)
        verify_fpak(blob)
        packs_meta.append({"path": pub, "sha256": sha, "bytes": len(blob), "group": group_name, "entries": len(rows)})
        for r in rows:
            base, variant = r["logical_id"].split("@", 1)
            entry_ref = {
                "pak": pub,
                "entry": r["logical_id"],
                "offset": r["offset"],
                "bytes": r["bytes"],
                "sha256": r["sha256"],
                "index_bytes": r["index_bytes"],
            }
            a = assets.setdefault(base, {"kind": "data", "tier": ["medium"], "placeholder": True})
            a[variant] = entry_ref

    for (gx, gz) in sorted(groups):
        gname = f"terrain_{map_id}_g_{gx}_{gz}"
        items = sorted(groups[(gx, gz)], key=lambda t: (t[1], t[0]))
        batch: list[tuple[str, int, int, bytes]] = []
        acc_bytes = HEADER_BYTES
        for bx, bz, bpath, cpath in items:
            bdata, cdata = bpath.read_bytes(), cpath.read_bytes()
            base = f"terrain_{map_id}_b_{bx}_{bz}"
            pair = [
                (f"{base}@chunk", KIND_META, CODEC_RAW, bdata),
                (f"{base}@collision", KIND_META, CODEC_RAW, cdata),
            ]
            pair_sz = sum(align_up(len(x[3])) for x in pair)
            if batch and acc_bytes + pair_sz > max_pack_bytes:
                flush(gname, batch)
                batch, acc_bytes = [], HEADER_BYTES
            batch.extend(pair)
            acc_bytes += pair_sz
        flush(gname, batch)

    standalone = find_standalone(fpak_out)
    if standalone:
        raise RuntimeError(f"standalone files found under {fpak_out}: {standalone}")

    manifest = {
        "schema": 1,
        "build": DETERMINISTIC_BUILD_TS,
        "game_version": "0.1.0",
        "data_hash": "",
        "assets": dict(sorted(assets.items())),
        "bundles": {f"map_{map_id}_medium": [f"terrain_{map_id}_*"]},
        "budgets": {f"map_{map_id}_medium_bytes": MAP_CARENTAN_MEDIUM_BUDGET},
    }
    manifest_text = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    manifest_out.parent.mkdir(parents=True, exist_ok=True)
    manifest_out.write_text(manifest_text, encoding="utf-8")

    # Verify consistency across all manifest entries against the written .fpak files
    pack_cache: dict[str, dict[str, dict]] = {}
    for base, asset in manifest["assets"].items():
        for field in ("chunk", "collision"):
            ref = asset[field]
            pak_rel = ref["pak"]
            if pak_rel not in pack_cache:
                rows = verify_fpak((fpak_out / pak_rel.lstrip("/")).read_bytes())
                pack_cache[pak_rel] = {r["logical_id"]: r for r in rows}
            idx = pack_cache[pak_rel][ref["entry"]]
            assert idx["offset"] == ref["offset"] and idx["bytes"] == ref["bytes"] and idx["sha256"] == ref["sha256"]

    pack_sizes = [p["bytes"] for p in packs_meta]
    return {
        "packs": len(packs_meta),
        "chunks_packed": len(assets),
        "entries_packed": sum(p["entries"] for p in packs_meta),
        "total_fpak_bytes": sum(pack_sizes),
        "min_pack_bytes": min(pack_sizes),
        "max_pack_bytes": max(pack_sizes),
        "manifest_sha256": hashlib.sha256(manifest_text.encode("utf-8")).hexdigest(),
        "packs_sha256": hashlib.sha256(
            json.dumps([(p["path"], p["sha256"]) for p in packs_meta]).encode("utf-8")
        ).hexdigest(),
        "assert_no_standalone": True,
        "consistency_ok": True,
    }
