"""Synthetic video / audio generators with exactly known ground truth (no reference-game data)."""

from __future__ import annotations

import subprocess
from pathlib import Path

import cv2
import numpy as np
from scipy.io import wavfile


def write_video(path, frames, fps: int, w: int, h: int, codec: str = "ffv1", audio: str | None = None,
                crf: int = 10) -> Path:
    """Encode an iterable of BGR uint8 (h,w,3) frames. ffv1 = lossless; libx264 = lossy."""
    path = Path(path)
    cmd = ["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "bgr24", "-s", f"{w}x{h}",
           "-r", str(fps), "-i", "-"]
    if audio:
        cmd += ["-i", str(audio)]
    if codec == "ffv1":
        cmd += ["-c:v", "ffv1"]
    else:
        cmd += ["-c:v", "libx264", "-crf", str(crf), "-preset", "fast", "-pix_fmt", "yuv420p"]
    if audio:
        cmd += ["-c:a", "pcm_s16le", "-shortest"]
    cmd += [str(path)]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    for f in frames:
        p.stdin.write(np.ascontiguousarray(f, dtype=np.uint8).tobytes())
    p.stdin.close()
    err = p.stderr.read().decode()
    if p.wait() != 0:
        raise RuntimeError(err)
    return path


def gunshot_audio(times_s, duration_s: float, rate: int = 48000, snr_db: float = 25.0,
                  echo_delay_s: float = 0.03, echo_gain: float = 0.35, seed: int = 1,
                  first_louder: bool = True) -> np.ndarray:
    """Float waveform: each shot = exponentially decaying band-limited noise burst (+ delayed echo) over
    white noise at `snr_db` (relative to a unit-amplitude shot's RMS over its first 20 ms)."""
    rng = np.random.default_rng(seed)
    n = int(duration_s * rate)
    x = np.zeros(n)
    L = int(0.06 * rate)
    env = np.exp(-np.arange(L) / (0.012 * rate))
    for k, t in enumerate(times_s):
        i = int(round(t * rate))
        burst = rng.standard_normal(L) * env
        # crude low-pass to avoid pure white bursts
        burst = np.convolve(burst, np.ones(4) / 4, mode="same")
        g = 1.4 if (k == 0 and first_louder) else 1.0
        for delay, gain in ((0.0, g), (echo_delay_s, echo_gain * g)):
            j = i + int(delay * rate)
            if j < n:
                m = min(L, n - j)
                x[j:j + m] += gain * burst[:m]
    sig_rms = 0.1  # reference level for the noise ratio
    x += rng.standard_normal(n) * sig_rms * 10 ** (-snr_db / 20.0) * 3
    return x / max(1.0, np.abs(x).max()) * 0.9


def write_wav(path, x: np.ndarray, rate: int = 48000) -> Path:
    wavfile.write(str(path), rate, (np.clip(x, -1, 1) * 32767).astype(np.int16))
    return Path(path)


def flash_video(path, onsets, n_frames: int, fps: int, w: int = 320, h: int = 180, flash_len: int = 2,
                codec: str = "ffv1") -> Path:
    """Dark scene with a bright muzzle-flash square in the lower-right region at the given first frames."""
    rng = np.random.default_rng(3)
    base = (rng.random((h, w)) * 40 + 20).astype(np.uint8)
    on = set()
    for o in onsets:
        for k in range(flash_len):
            on.add(o + k)

    def gen():
        for i in range(n_frames):
            f = np.dstack([base] * 3).copy()
            if i in on:
                f[h // 2:h // 2 + 40, w // 2:w // 2 + 60] = 250
            yield f
    return write_video(path, gen(), fps, w, h, codec)


def moving_block_video(path, speed_mps: float, px_per_m: float, fps: int, n_frames: int,
                       w: int = 1280, h: int = 240, start_px: float = 40.0, codec: str = "ffv1",
                       profile=None) -> tuple[Path, np.ndarray]:
    """White block on a textured background moving at `speed_mps` (px_per_m scale); `profile(t) -> x_px`
    overrides the constant-speed motion. Returns (video, true x of the block centre per frame)."""
    rng = np.random.default_rng(5)
    bg = cv2.GaussianBlur((rng.random((h, w)) * 90 + 30).astype(np.uint8), (0, 0), 1.2)
    true_x = np.zeros(n_frames)

    def gen():
        for i in range(n_frames):
            t = i / fps
            x = profile(t) if profile else start_px + speed_mps * px_per_m * t
            true_x[i] = x
            f = np.dstack([bg] * 3).copy()
            # sub-pixel placement through an anti-aliased rectangle (x in [x-8, x+8))
            x0, x1 = x - 8.0, x + 8.0
            for px in range(int(np.floor(x0)), int(np.ceil(x1))):
                cov = max(0.0, min(px + 1, x1) - max(px, x0))
                if 0 <= px < w:
                    f[h // 2 - 10:h // 2 + 10, px] = (f[h // 2 - 10:h // 2 + 10, px] * (1 - cov) + 255 * cov).astype(np.uint8)
            yield f
    write_video(path, gen(), fps, w, h, codec)
    return Path(path), true_x
