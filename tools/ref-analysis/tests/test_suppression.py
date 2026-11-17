import cv2
import numpy as np
import pytest

from ref_analysis import suppression as sp
from synth import write_video

pytestmark = pytest.mark.ffmpeg
FPS = 120
W, H = 320, 180


def texture(seed=2):
    rng = np.random.default_rng(seed)
    img = (rng.random((H, W)) * 255).astype(np.uint8)
    img = cv2.GaussianBlur(img, (0, 0), 1.0)
    cv2.circle(img, (160, 90), 30, 255, 2)
    cv2.rectangle(img, (40, 30), (110, 80), 40, -1)
    return img


def test_blur_curve_keypoints_match_ground_truth(tmp_path):
    img = texture()
    n, trig = 300, 100

    def sigma(i):
        if i < trig:
            return 0.0
        if i < trig + 12:
            return 3.0 * (i - trig) / 12
        if i < trig + 60:
            return 3.0
        if i < trig + 84:
            return 3.0 * (1 - (i - trig - 60) / 24)
        return 0.0

    frames = []
    for i in range(n):
        s = sigma(i)
        g = cv2.GaussianBlur(img, (0, 0), s) if s > 0.05 else img
        frames.append(g)
    truth = np.array([sp.laplacian_variance(g) for g in frames])
    v = write_video(tmp_path / "b.mkv", (np.dstack([g] * 3) for g in frames), FPS, W, H)
    got = sp.sharpness_curve(v)
    assert len(got) == n
    assert np.allclose(got, truth, rtol=1e-6)         # lossless chain reproduces the curve exactly
    k_t = sp.sharp_keypoints(truth, (0, 80), FPS, trig)
    k_g = sp.sharp_keypoints(got, (0, 80), FPS, trig)
    print(f"onset {k_g.onset_delay_s * 1000:.1f} ms, peak {k_g.peak_norm:.4f}, recovery {k_g.recovery_time_s * 1000:.1f} ms")
    assert k_g.onset_frame == k_t.onset_frame and k_g.recovery_frame == k_t.recovery_frame
    assert k_g.peak_norm == pytest.approx(k_t.peak_norm, rel=1e-6)
    assert k_g.peak_norm < 0.15 and k_g.onset_delay_s is not None
    # ground truth from the generator: sigma first exceeds ~0.1 within 1 frame after the trigger
    assert (k_g.onset_frame - trig) <= 3
    assert k_g.unc_s == pytest.approx(1 / FPS)


def test_lossy_encoding_keeps_keypoints_within_a_frame(tmp_path):
    img = texture()
    n, trig = 200, 80
    frames = [cv2.GaussianBlur(img, (0, 0), 2.5) if trig <= i < trig + 50 else img for i in range(n)]
    truth = np.array([sp.laplacian_variance(g) for g in frames])
    v = write_video(tmp_path / "b.mp4", (np.dstack([g] * 3) for g in frames), FPS, W, H, codec="libx264", crf=14)
    got = sp.sharpness_curve(v)
    kt = sp.sharp_keypoints(truth, (0, 60), FPS, trig)
    kg = sp.sharp_keypoints(got, (0, 60), FPS, trig)
    assert abs(kg.onset_frame - kt.onset_frame) <= 1
    assert abs(kg.recovery_frame - kt.recovery_frame) <= 1


def test_no_suppression_gives_no_onset():
    s = np.ones(100) * 50.0
    k = sp.sharp_keypoints(s, (0, 40), FPS, 50)
    assert k.onset_frame is None and k.onset_delay_s is None and k.peak_norm == 1.0


def test_shake_amplitude_and_frequency(tmp_path):
    img = cv2.resize(texture(), (W, H))
    amp, f_hz, n = 4.0, 6.0, 480
    a0, a1 = 100, 380
    big = cv2.copyMakeBorder(img, 20, 20, 20, 20, cv2.BORDER_REFLECT)

    def gen():
        for i in range(n):
            dx = amp * np.sin(2 * np.pi * f_hz * (i - a0) / FPS) if a0 <= i < a1 else 0.0
            m = np.float32([[1, 0, 20 + dx], [0, 1, 20]])
            g = cv2.warpAffine(big, m, (W + 40, H + 40), flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_REFLECT)[20:20 + H, 20:20 + W]
            yield np.dstack([g] * 3)
    v = write_video(tmp_path / "s.mkv", gen(), FPS, W, H)
    shift = sp.flow_shift_curve(v, scale=(W, H), roi_frac=0.9)
    assert shift.shape == (n - 1, 2)
    m = sp.shake_metrics(shift, FPS)
    # image content moves +dx, the camera "position" is the cumulative flow: p2p = 2 A
    print(f"shake: p2p {m.p2p_px:.3f} px (true {2 * amp}), f {m.dominant_hz:.2f} Hz (true {f_hz}), onset frame {m.onset_frame}")
    assert abs(m.p2p_px - 2 * amp) / (2 * amp) <= 0.10
    assert abs(m.dominant_hz - f_hz) <= 0.3
    assert m.onset_frame is not None and abs(m.onset_frame - a0) <= 10  # first pixels of motion ramp up slowly


def test_vignette_and_saturation_curves(tmp_path):
    n = 40

    def gen():
        for i in range(n):
            f = np.full((H, W, 3), 128, np.uint8)
            if i < 30:   # red cast (higher saturation) until frame 30
                f[..., 2] = 188
            if i >= 20:  # edges darkened to 50 % from frame 20
                m = np.ones((H, W))
                m[:30] = m[-30:] = 0.5
                m[:, :30] = m[:, -30:] = 0.5
                f = (f * m[..., None]).astype(np.uint8)
            yield f
    v = write_video(tmp_path / "v.mkv", gen(), FPS, W, H)
    vg = sp.vignette_curve(v, edge_frac=0.1)
    assert vg[5] == pytest.approx(1.0, abs=0.03)
    assert vg[25] == pytest.approx(0.5, abs=0.05)
    sat = sp.saturation_curve(v)
    assert sat[5] > sat[35] + 20
