"""64 m chunk slicing (MAPS 6.1, ARCHITECTURE 9.1/9.5). Neighbouring chunks share edge samples."""
from __future__ import annotations

import struct
from pathlib import Path

import numpy as np

CHUNK_M = 64
CHUNK_SAMPLES = 65
GRID = 4
MAGIC = b"FCOL"
C_VERSION = 1
HEADER_FMT = "<4sBBHff"  # magic, version, pad, samples, hmin_m, hmax_m
HEADER_BYTES = struct.calcsize(HEADER_FMT)  # 16
B_BYTES = CHUNK_SAMPLES * CHUNK_SAMPLES * 2  # 8450
C_BYTES = HEADER_BYTES + B_BYTES + CHUNK_M * CHUNK_M  # 12562


def chunk_slice(q: np.ndarray, bx: int, bz: int) -> np.ndarray:
    if not (0 <= bx < GRID and 0 <= bz < GRID):
        raise IndexError(f"chunk index out of range: {bx},{bz}")
    x0, z0 = bx * CHUNK_M, bz * CHUNK_M
    return np.ascontiguousarray(q[z0 : z0 + CHUNK_SAMPLES, x0 : x0 + CHUNK_SAMPLES])


def encode_b(sl: np.ndarray) -> bytes:
    return sl.astype("<u2").tobytes()


def encode_c(sl: np.ndarray, hmin: float, hmax: float) -> bytes:
    head = struct.pack(HEADER_FMT, MAGIC, C_VERSION, 0, CHUNK_SAMPLES, hmin, hmax)
    return head + encode_b(sl) + bytes(CHUNK_M * CHUNK_M)  # material ids: M0 all 0


def decode_c(data: bytes):
    if len(data) != C_BYTES:
        raise ValueError(f"collision chunk must be {C_BYTES} bytes, got {len(data)}")
    magic, ver, _pad, n, hmin, hmax = struct.unpack_from(HEADER_FMT, data)
    if magic != MAGIC or ver != C_VERSION or n != CHUNK_SAMPLES:
        raise ValueError("bad collision chunk header")
    h = np.frombuffer(data, dtype="<u2", count=n * n, offset=HEADER_BYTES).reshape(n, n)
    mats = np.frombuffer(data, dtype=np.uint8, offset=HEADER_BYTES + B_BYTES).reshape(CHUNK_M, CHUNK_M)
    return hmin, hmax, h, mats


def write_chunks(out: Path, q: np.ndarray, hmin: float, hmax: float) -> list[str]:
    d = out / "chunks"
    d.mkdir(parents=True, exist_ok=True)
    rels = []
    for bz in range(GRID):
        for bx in range(GRID):
            sl = chunk_slice(q, bx, bz)
            (d / f"b_{bx}_{bz}.bin").write_bytes(encode_b(sl))
            (d / f"c_{bx}_{bz}.bin").write_bytes(encode_c(sl, hmin, hmax))
            rels += [f"chunks/b_{bx}_{bz}.bin", f"chunks/c_{bx}_{bz}.bin"]
    return rels
