"""ffmpeg frame / audio extraction (REFERENCE_MEASUREMENTS 3.2, 3.3). Output is private, never committed."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Iterator

import numpy as np

from .common import ToolError, need, probe

Crop = tuple[int, int, int, int]  # W, H, X, Y


def _vf(crop: Crop | None, scale: tuple[int, int] | None) -> list[str]:
    parts = []
    if crop:
        w, h, x, y = crop
        parts.append(f"crop={w}:{h}:{x}:{y}")
    if scale:
        parts.append(f"scale={scale[0]}:{scale[1]}:flags=area")
    return ["-vf", ",".join(parts)] if parts else []


def iter_frames(
    path: str | Path,
    crop: Crop | None = None,
    gray: bool = True,
    scale: tuple[int, int] | None = None,
    t0: float | None = None,
    t1: float | None = None,
) -> Iterator[tuple[int, np.ndarray]]:
    """Yield (frame_index, array) for every frame, in memory (no PNGs).

    Frames are never duplicated or dropped (`-fps_mode passthrough`). Index 0 is the first frame of the
    requested window; pass `t0` to know the absolute offset yourself (`round(t0 * fps)`)."""
    info = probe(path)
    w, h = info.width, info.height
    if crop:
        w, h = crop[0], crop[1]
    if scale:
        w, h = scale
    pix = "gray" if gray else "bgr24"
    cmd = [need("ffmpeg"), "-v", "error"]
    if t0 is not None:
        cmd += ["-ss", f"{t0}"]
    if t1 is not None:
        cmd += ["-to", f"{t1}"]
    cmd += ["-i", str(path), "-an", *_vf(crop, scale), "-fps_mode", "passthrough",
            "-f", "rawvideo", "-pix_fmt", pix, "-"]
    ch = 1 if gray else 3
    size = w * h * ch
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert p.stdout is not None
    i = 0
    early = True
    try:
        while True:
            buf = p.stdout.read(size)
            if len(buf) < size:
                break
            a = np.frombuffer(buf, dtype=np.uint8)
            yield i, (a.reshape(h, w) if gray else a.reshape(h, w, 3))
            i += 1
        early = False
    finally:
        if early:  # consumer stopped reading: do not wait for ffmpeg to finish
            p.kill()
        p.stdout.close()
        err = p.stderr.read().decode(errors="replace") if p.stderr else ""
        p.wait()
        if not early and p.returncode != 0 and i == 0:
            raise ToolError(f"ffmpeg failed: {err.strip()}")


def count_frames(path: str | Path) -> int:
    n = 0
    for _ in iter_frames(path, gray=True, scale=(16, 16)):
        n += 1
    return n


def extract_pngs(
    path: str | Path, out_dir: str | Path, crop: Crop | None = None,
    t0: float | None = None, t1: float | None = None,
) -> int:
    """Write `%06d.png` (private disk only); returns the number of frames written."""
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    cmd = [need("ffmpeg"), "-v", "error", "-y"]
    if t0 is not None:
        cmd += ["-ss", f"{t0}"]
    if t1 is not None:
        cmd += ["-to", f"{t1}"]
    cmd += ["-i", str(path), *_vf(crop, None), "-fps_mode", "passthrough", str(out / "%06d.png")]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        raise ToolError(r.stderr.strip())
    return len(list(out.glob("*.png")))


def extract_audio(path: str | Path, out_wav: str | Path, rate: int = 48000) -> Path:
    """`ffmpeg -i in -vn -ac 1 -ar 48000 audio.wav` (3.3 step 2); internal analysis only."""
    out = Path(out_wav)
    out.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [need("ffmpeg"), "-v", "error", "-y", "-i", str(path), "-vn", "-ac", "1", "-ar", str(rate),
         "-c:a", "pcm_s16le", str(out)],
        capture_output=True, text=True)
    if r.returncode != 0:
        raise ToolError(r.stderr.strip())
    return out
