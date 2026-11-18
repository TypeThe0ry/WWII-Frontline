"""M1-035 chunk baking: height, splat, collision, props, vegetation and hedge segments per 64 m chunk.
Container format FCHK v1 is documented in `chunks/FORMAT.txt` (written by `write_format_doc`)."""
from __future__ import annotations

import hashlib
import json
import math
import struct
import zlib
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

from . import buildings_annotate as ba, chunks as ck, export, hedgerow_tools as ht, splat, vegetation

MAP_ID = "carentan"
FORMAT_VERSION = 1
BX_RANGE = (5, 24)
BZ_RANGE = (8, 26)
MAX_CHUNK_BYTES = 81920
S_HEIGHT, S_SPLAT, S_PROPS, S_VEG, S_HEDGE = 1, 2, 3, 4, 5
SECTION_NAMES = {S_HEIGHT: "height", S_SPLAT: "splat", S_PROPS: "props", S_VEG: "veg", S_HEDGE: "hedge_segments"}
ZLIB_LEVEL = 9
_CTX: dict = {}


def delta_encode(q: np.ndarray) -> bytes:
    """Row-wise delta (wrapping u16), split into low/high byte planes, deflated."""
    d = np.diff(q.astype(np.int32), axis=1, prepend=0).astype(np.uint16)
    d[:, 0] = q[:, 0]
    d[1:, 0] = (q[1:, 0].astype(np.int32) - q[:-1, 0].astype(np.int32)).astype(np.uint16)
    return zlib.compress((d & 0xFF).astype(np.uint8).tobytes() + (d >> 8).astype(np.uint8).tobytes(), ZLIB_LEVEL)


def delta_decode(blob: bytes, n=ck.CHUNK_SAMPLES) -> np.ndarray:
    raw = np.frombuffer(zlib.decompress(blob), dtype=np.uint8)
    lo, hi = raw[: n * n].reshape(n, n).astype(np.uint16), raw[n * n :].reshape(n, n).astype(np.uint16)
    d = lo | (hi << 8)
    q = np.zeros((n, n), dtype=np.uint16)
    q[0, 0] = d[0, 0]
    for r in range(1, n):
        q[r, 0] = (int(q[r - 1, 0]) + int(d[r, 0])) & 0xFFFF
    for c in range(1, n):
        q[:, c] = (q[:, c - 1].astype(np.int32) + d[:, c].astype(np.int32)).astype(np.uint16) & 0xFFFF
    return q


def pack_container(bx, bz, sections: dict) -> bytes:
    ids = sorted(sections)
    head = struct.pack("<4sBBHHH", b"FCHK", FORMAT_VERSION, 0, bx, bz, len(ids))
    table_len = len(ids) * 16
    off = len(head) + table_len
    table, body = b"", b""
    for sid in ids:
        data, raw_len = sections[sid]
        table += struct.pack("<BBHIII", sid, 1, 0, off + len(body), len(data), raw_len)
        body += data
    return head + table + body


def unpack_container(blob: bytes) -> dict:
    magic, ver, _f, bx, bz, n = struct.unpack_from("<4sBBHHH", blob)
    if magic != b"FCHK" or ver != FORMAT_VERSION:
        raise ValueError("bad chunk header")
    out = {"bx": bx, "bz": bz, "sections": {}}
    for i in range(n):
        sid, codec, _p, off, clen, rlen = struct.unpack_from("<BBHIII", blob, 12 + i * 16)
        out["sections"][sid] = (blob[off : off + clen], rlen)
    return out


def _cm(v):
    return int(round(v * 100))


def props_for(bx, bz, buildings, landmarks):
    x0, z0 = bx * 64.0, bz * 64.0
    items = []
    for f in buildings:
        ring = f["geometry"]["coordinates"][0]
        cx, cy = ba.centroid(ring)
        if not (x0 <= cx < x0 + 64 and z0 <= cy < z0 + 64):
            continue
        p = f["properties"]
        xs, ys = [q[0] for q in ring], [q[1] for q in ring]
        spans = sorted({(int(x // 64), int(y // 64)) for x, y in ((min(xs), min(ys)), (max(xs), max(ys)))} - {(bx, bz)})
        items.append({"id": "bld_" + p["_osm_id"].replace("/", "_"), "fp_cm": [[_cm(x - x0), _cm(y - z0)] for x, y in ring[:-1]],
                      "style": p["style"], "levels": p["levels"], "enterable": p["enterable"],
                      "door": None if not p.get("door") else [_cm(p["door"]["center"][0] - x0), _cm(p["door"]["center"][1] - z0)],
                      "ref": "PH_" + str(p["asset_ref"]), "spans": [list(s) for s in spans]})
    for m in landmarks:
        if m.get("found_in_osm") and x0 <= m["pos_e"] < x0 + 64 and z0 <= m["pos_n"] < z0 + 64:
            items.append({"id": "lm_" + m["id"], "pos_cm": [_cm(m["pos_e"] - x0), _cm(m["pos_n"] - z0)],
                          "dims_m": m["placeholder_dims_m"], "ref": "PH_" + str(m["asset_ref"])})
    items.sort(key=lambda d: d["id"])
    return json.dumps(items, separators=(",", ":"), sort_keys=True).encode()


def hedge_segments(bx, bz, hedges):
    x0, z0 = bx * 64.0, bz * 64.0
    segs = []
    for h in hedges:
        local = [(x - x0, y - z0) for x, y in h["pts"]]
        from .clean import clip_line
        for piece in clip_line(local, 0.0, 64.0):
            pts = ht.resample(piece, 2.0)
            segs.append((h["height_m"], h["mound_m"], h["blocking"], pts))
    out = struct.pack("<H", len(segs))
    for hh, mm, bl, pts in segs:
        out += struct.pack("<BBBB", min(255, int(hh * 10)), min(255, int(mm * 10)), 1 if bl else 0, min(255, len(pts)))
        for x, y in pts[:255]:
            out += struct.pack("<HH", vegetation._q(x), vegetation._q(y))
    return out


def _veg_bytes(trees, shrubs, grass_seed, grass_count):
    out = struct.pack("<HHIH", len(trees), len(shrubs), grass_seed, grass_count)
    for t in trees + shrubs:
        out += struct.pack("<HHBBB", *t)
    return out


def init_worker(ctx):
    _CTX.update(ctx)


def bake_chunk(key):
    bx, bz = key
    c = _CTX
    q = c["q"]
    sl = np.ascontiguousarray(q[bz * 64 : bz * 64 + 65, bx * 64 : bx * 64 + 65])
    pad = splat.HALO
    x0, z0 = bx * 64 - pad, bz * 64 - pad
    win = np.zeros((64 + 2 * pad,) * 2)
    r0, r1 = max(0, z0), min(q.shape[0], z0 + 64 + 2 * pad)
    c0, c1 = max(0, x0), min(q.shape[1], x0 + 64 + 2 * pad)
    win[:] = np.nan
    win[r0 - z0 : r1 - z0, c0 - x0 : c1 - x0] = export.dequantize(q[r0:r1, c0:c1], c["hmin"], c["hmax"])
    win = np.where(np.isnan(win), np.nanmean(win), win)
    w8, m, idx = splat.splat_chunk(bx, bz, c["layers"], win)
    planar = np.moveaxis(w8, -1, 0).tobytes()
    trees, shrubs, seed = vegetation.place(MAP_ID, bx, bz, m, c["layers"]["forests"], c["hedges"], 0.6)
    grass_frac = float((w8[..., splat.L["grass"]].astype(np.float64) / 255.0).mean())
    veg = _veg_bytes(trees, shrubs, seed, int(round(grass_frac * vegetation.MAX_GRASS)))
    props = props_for(bx, bz, c["layers"]["buildings"], c["landmarks"])
    hed = hedge_segments(bx, bz, c["hedges"])
    mats = np.take(idx[splat.HALO : splat.HALO + 64, splat.HALO : splat.HALO + 64], np.arange(64), axis=0).astype(np.uint8)
    col = ck.encode_c(sl, c["hmin"], c["hmax"])[: ck.HEADER_BYTES + ck.B_BYTES] + mats.tobytes()
    secs = {S_HEIGHT: (delta_encode(sl), sl.nbytes), S_SPLAT: (zlib.compress(planar, ZLIB_LEVEL), len(planar)),
            S_PROPS: (zlib.compress(props, ZLIB_LEVEL), len(props)), S_VEG: (zlib.compress(veg, ZLIB_LEVEL), len(veg)),
            S_HEDGE: (zlib.compress(hed, ZLIB_LEVEL), len(hed))}
    blob = pack_container(bx, bz, secs)
    cblob = zlib.compress(col, ZLIB_LEVEL)
    sizes = {SECTION_NAMES[k]: len(v[0]) for k, v in secs.items()}
    sizes["collision"] = len(cblob)
    layer_share = (w8.reshape(-1, 8).sum(axis=0) / (255.0 * 64 * 64)).round(5).tolist()
    return {"key": (bx, bz), "blob": blob, "cblob": cblob, "sizes": sizes, "trees": len(trees), "shrubs": len(shrubs),
            "layer_share": layer_share, "grass_count": int(round(grass_frac * vegetation.MAX_GRASS))}


def load_ctx(out: Path):
    meta = json.loads((out / "meta.json").read_text(encoding="utf-8"))
    hm = meta["height"]
    q = export.read_r16(out / "heightmap_1m.r16", hm["samples"])
    layers = {k: json.loads((out / f"{k}.geojson").read_text(encoding="utf-8"))["features"]
              for k in ("buildings", "roads", "water", "hedgerows", "forests")}
    hedges = [{"pts": [tuple(c) for c in f["geometry"]["coordinates"]], "height_m": f["properties"]["height_m"],
               "mound_m": f["properties"]["mound_width_m"], "blocking": f["properties"]["vehicle_blocking"]} for f in layers["hedgerows"]]
    lm = json.loads((out / "landmarks.json").read_text(encoding="utf-8"))["landmarks"]
    return {"q": q, "hmin": hm["hmin_m"], "hmax": hm["hmax_m"], "layers": layers, "hedges": hedges, "landmarks": lm}


def keys():
    return [(bx, bz) for bz in range(BZ_RANGE[0], BZ_RANGE[1] + 1) for bx in range(BX_RANGE[0], BX_RANGE[1] + 1)]


def bake(out: Path, dest: Path, workers: int = 4, subset=None):
    ctx = load_ctx(out)
    ks = subset or keys()
    dest.mkdir(parents=True, exist_ok=True)
    if workers > 1:
        with ProcessPoolExecutor(workers, initializer=init_worker, initargs=(ctx,)) as ex:
            res = list(ex.map(bake_chunk, ks, chunksize=8))
    else:
        init_worker(ctx)
        res = [bake_chunk(k) for k in ks]
    entries = []
    for r in sorted(res, key=lambda r: r["key"]):
        bx, bz = r["key"]
        (dest / f"b_{bx}_{bz}.fchk").write_bytes(r["blob"])
        (dest / f"c_{bx}_{bz}.fcol").write_bytes(r["cblob"])
        total = len(r["blob"]) + len(r["cblob"])
        r["total"] = total
        entries.append({"key": f"terrain_carentan/b_{bx}_{bz}", "bx": bx, "bz": bz, "files": [f"b_{bx}_{bz}.fchk", f"c_{bx}_{bz}.fcol"],
                        "bytes": total, "sha256": hashlib.sha256(r["blob"] + r["cblob"]).hexdigest(), "lod": [0],
                        "data_quality": "draft"})
    return ctx, res, entries


def report(res, entries) -> dict:
    tot = [r["total"] for r in res]
    arr = np.array(tot)
    over = [e["key"] for e in entries if e["bytes"] > MAX_CHUNK_BYTES]
    comp = {k: int(sum(r["sizes"][k] for r in res)) for k in res[0]["sizes"]}
    share = np.array([r["layer_share"] for r in res]).mean(axis=0)
    return {"schema": 1, "format_version": FORMAT_VERSION, "codec": "zlib-9 (brotli unavailable, PI-084)", "chunks": len(res),
            "bytes": {"total": int(arr.sum()), "p50": int(np.percentile(arr, 50)), "p99": int(np.percentile(arr, 99)), "max": int(arr.max())},
            "max_bytes_limit": MAX_CHUNK_BYTES, "chunks_over_limit": over, "section_totals": comp,
            "splat_layer_share": {n: round(float(s), 4) for n, s in zip(splat.LAYERS, share)},
            "vegetation": {"max_trees": max(r["trees"] for r in res), "max_shrubs": max(r["shrubs"] for r in res),
                           "max_grass": max(r["grass_count"] for r in res), "limits": [vegetation.MAX_TREES, vegetation.MAX_SHRUBS, vegetation.MAX_GRASS]},
            "data_quality": "draft"}


FORMAT_DOC = """FCHK v1 (M1-035) - one file per 64 m chunk: b_{bx}_{bz}.fchk (visual+data), c_{bx}_{bz}.fcol (collision)
Header   : '<4sBBHHH' magic 'FCHK', version, flags, bx, bz, nsections
Table    : nsections x '<BBHIII' (id, codec(1=zlib), pad, offset, compressed_len, raw_len)
Sections : 1 height  : 65x65 u16, row-wise delta (wrapping), low-byte plane then high-byte plane, zlib
           2 splat   : 8 planes x 64x64 u8 (grass,dirt,mud,road,cobble,rock,sand,forest_floor), per-pixel sum 255, zlib
           3 props   : JSON list (bld_*/lm_*), footprints in chunk-local centimetres, PH_ asset refs, spans[] neighbours
           4 veg     : '<HHIH' ntrees nshrubs grass_seed grass_count, then (ntrees+nshrubs) x '<HHBBB' (x,z u16 of 64 m, species, scale, rot), zlib
           5 hedge   : '<H' nseg; each '<BBBB' height_dm mound_dm blocking npts + npts x '<HH' (u16 of 64 m), 2 m spacing, zlib
Collision: zlib of  <4sBBHff header (FCOL v1, 65 samples, hmin, hmax) + 65x65 u16 heights (same samples as the visual chunk)
           + 64x64 u8 material ids (= dominant splat layer index).
Height triangulation: every 1 m quad is split along the diagonal (x,z)-(x+1,z+1); the visual (CDLOD LOD0) and the
           collision height field must use this same diagonal (PI-084). Chunk edge samples are shared with neighbours.
"""


def tri_height(h: np.ndarray, x: float, z: float) -> float:
    """Height at chunk-local (x, z) m on the 1 m grid, triangles split along (0,0)-(1,1)."""
    i, j = min(int(z), h.shape[0] - 2), min(int(x), h.shape[1] - 2)
    fx, fz = x - j, z - i
    h00, h10, h01, h11 = h[i, j], h[i, j + 1], h[i + 1, j], h[i + 1, j + 1]
    if fx >= fz:
        return h00 + (h10 - h00) * fx + (h11 - h10) * fz
    return h00 + (h11 - h01) * fx + (h01 - h00) * fz


def verify(ctx, res, dest: Path, seed=1) -> dict:
    """Acceptance checks 4-6: raycast agreement, shared edges, splat sums, vegetation exclusions."""
    import random
    rng = random.Random(seed)
    by = {r["key"]: r for r in res}
    ks = sorted(by)
    q = ctx["q"]
    # 4: visual vs collision height (both decoded from the baked files)
    dmax = dsum = 0.0
    for _ in range(200):
        bx, bz = rng.choice(ks)
        x, z = rng.uniform(0, 64), rng.uniform(0, 64)
        vis = delta_decode(unpack_container(by[(bx, bz)]["blob"])["sections"][S_HEIGHT][0])
        col = zlib.decompress(by[(bx, bz)]["cblob"])
        ch = np.frombuffer(col, dtype="<u2", count=65 * 65, offset=ck.HEADER_BYTES).reshape(65, 65)
        a = export.dequantize(vis, ctx["hmin"], ctx["hmax"])
        b = export.dequantize(ch, ctx["hmin"], ctx["hmax"])
        d = abs(tri_height(a, x, z) - tri_height(b, x, z))
        dmax, dsum = max(dmax, d), dsum + d
    # 5: shared edges
    bad_edges = 0
    pairs = [(k, (k[0] + 1, k[1])) for k in ks if (k[0] + 1, k[1]) in by] + [(k, (k[0], k[1] + 1)) for k in ks if (k[0], k[1] + 1) in by]
    for k, n in rng.sample(pairs, min(100, len(pairs))):
        a = delta_decode(unpack_container(by[k]["blob"])["sections"][S_HEIGHT][0])
        b = delta_decode(unpack_container(by[n]["blob"])["sections"][S_HEIGHT][0])
        if n[0] > k[0]:
            bad_edges += int(not np.array_equal(a[:, 64], b[:, 0]))
        else:
            bad_edges += int(not np.array_equal(a[64, :], b[0, :]))
    # splat sums
    bad_sum = 0
    for k in rng.sample(ks, min(40, len(ks))):
        sp = np.frombuffer(zlib.decompress(unpack_container(by[k]["blob"])["sections"][S_SPLAT][0]), dtype=np.uint8).reshape(8, 64, 64)
        bad_sum += int((np.abs(sp.sum(axis=0, dtype=np.int32) - 255) > 1).sum())
    # 6: vegetation exclusions on 10k instances
    viol = checked = 0
    for k in ks:
        veg = zlib.decompress(unpack_container(by[k]["blob"])["sections"][S_VEG][0])
        nt, ns, _seed, _g = struct.unpack_from("<HHIH", veg)
        X, Y = splat.window(k[0], k[1])
        m = splat.masks(X, Y, ctx["layers"])
        for i in range(nt + ns):
            x, z, _sp, _sc, _r = struct.unpack_from("<HHBBB", veg, 10 + i * 7)
            checked += 1
            if not vegetation._allowed(m, k[0], k[1], k[0] * 64 + x / 65535 * 64, k[1] * 64 + z / 65535 * 64):
                viol += 1
        if checked >= 10000:
            break
    return {"raycast_max_dh_m": round(dmax, 5), "raycast_mean_dh_m": round(dsum / 200, 5), "edge_pairs_checked": min(100, len(pairs)),
            "edge_mismatches": bad_edges, "splat_bad_pixels": bad_sum, "veg_checked": checked, "veg_violations": viol}


def index_doc(entries) -> dict:
    return {"schema": "frontline.map.chunk_index/1", "map_id": MAP_ID, "format_version": FORMAT_VERSION, "bundle": "map_carentan_medium",
            "chunk_m": 64, "bx_range": list(BX_RANGE), "bz_range": list(BZ_RANGE), "count": len(entries), "chunks": entries}
