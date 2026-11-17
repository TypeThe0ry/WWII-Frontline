"""ffprobe wrapper and frame/time conversion (REFERENCE_MEASUREMENTS 2.2)."""

from __future__ import annotations

import json
import shutil
import subprocess
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path


class ToolError(RuntimeError):
    pass


def need(binary: str) -> str:
    p = shutil.which(binary)
    if not p:
        raise ToolError(f"{binary} not found on PATH (install ffmpeg >= 6)")
    return p


def frac(s: str) -> Fraction:
    if not s or s == "0/0":
        return Fraction(0)
    return Fraction(s)


@dataclass(frozen=True)
class ProbeInfo:
    path: str
    width: int
    height: int
    fps: Fraction  # r_frame_rate
    avg_fps: Fraction
    duration_s: float
    nb_frames: int | None
    has_audio: bool
    audio_rate: int | None

    @property
    def is_cfr(self) -> bool:
        """2.2: avg_frame_rate == r_frame_rate; otherwise the capture must be discarded."""
        return self.fps == self.avg_fps and self.fps > 0

    @property
    def frame_unc_s(self) -> float:
        return 1.0 / float(self.fps)


def probe(path: str | Path) -> ProbeInfo:
    out = subprocess.run(
        [need("ffprobe"), "-v", "error", "-print_format", "json", "-show_streams",
         "-show_format", str(path)],
        capture_output=True, text=True, check=False,
    )
    if out.returncode != 0:
        raise ToolError(f"ffprobe failed for {path}: {out.stderr.strip()}")
    j = json.loads(out.stdout)
    v = next((s for s in j["streams"] if s["codec_type"] == "video"), None)
    if v is None:
        raise ToolError(f"{path}: no video stream")
    a = next((s for s in j["streams"] if s["codec_type"] == "audio"), None)
    dur = float(v.get("duration") or j["format"].get("duration") or 0.0)
    nb = v.get("nb_frames")
    return ProbeInfo(
        path=str(path),
        width=int(v["width"]),
        height=int(v["height"]),
        fps=frac(v["r_frame_rate"]),
        avg_fps=frac(v["avg_frame_rate"]),
        duration_s=dur,
        nb_frames=int(nb) if nb not in (None, "N/A") else None,
        has_audio=a is not None,
        audio_rate=int(a["sample_rate"]) if a else None,
    )


def frame_to_time(n: int | float, fps: float | Fraction) -> float:
    """t = n / fps (frame 0 is the first video frame)."""
    return float(n) / float(fps)


def interval_s(n1: int | float, n2: int | float, fps: float | Fraction) -> tuple[float, float]:
    """(dt, +-uncertainty) between two event frames; uncertainty is +-1 frame (2.2)."""
    f = float(fps)
    return (n2 - n1) / f, 1.0 / f
