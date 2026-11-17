import math

import cv2
import numpy as np
import pytest

from ref_analysis import calibrate as cal
from synth import moving_block_video, write_video

pytestmark = pytest.mark.ffmpeg
FPS = 120


def test_speed_from_marks_error_propagation():
    # 100 m in 22.7273 s = 4.4 m/s; 120 fps clock
    n = round(22.7273 * FPS)
    r = cal.speed_from_marks(60, 60 + n, 100.0, FPS, distance_unc_m=0.5)
    assert r.speed_mps == pytest.approx(4.4, rel=2e-4)
    assert r.uncertainty_mps / r.speed_mps == pytest.approx(math.hypot(0.005, (1 / FPS) / r.dt_s), rel=1e-6)
    assert r.uncertainty_mps / r.speed_mps < 0.0051
    with pytest.raises(ValueError):
        cal.speed_from_marks(10, 10, 5.0, FPS)


@pytest.mark.parametrize("codec", ["ffv1", "libx264"])
def test_constant_speed_4_4_mps_blob_track(tmp_path, codec):
    px_per_m = 100.0
    v, true_x = moving_block_video(tmp_path / "m.mkv", 4.4, px_per_m, FPS, 240, codec=codec)
    t = cal.track_bright_blob(v, thresh=200)
    assert t.shape == (240, 3)
    got = cal.fit_speed(t[:, 0], t[:, 1], FPS, px_per_m)
    err = abs(got - 4.4) / 4.4 * 100
    print(f"4.4 m/s block ({codec}): measured {got:.4f} m/s, err {err:.3f} %")
    assert err <= 3.0
    assert err <= 0.5  # in practice far better than the +-3 % acceptance
    # tracked centre follows the true centre to < 0.5 px
    assert np.abs(t[:, 1] - true_x - 0.0).max() < 1.0


def test_template_track_subpixel_speed(tmp_path):
    px_per_m = 100.0
    v, _ = moving_block_video(tmp_path / "m.mkv", 4.4, px_per_m, FPS, 240)
    first = next(iter(__import__("ref_analysis.extract_frames", fromlist=["x"]).iter_frames(v)))[1]
    cx = 40
    tpl = first[120 - 25:120 + 25, cx - 25:cx + 25].copy()
    t = cal.track_template(v, tpl, roi=(1280, 100, 0, 70))
    got = cal.fit_speed(t[:, 0], t[:, 1], FPS, px_per_m)
    err = abs(got - 4.4) / 4.4 * 100
    print(f"4.4 m/s template track: {got:.4f} m/s, err {err:.3f} %")
    assert err <= 3.0


def test_marks_method_on_video_events(tmp_path):
    # the block crosses two 'grid lines' 4.4 m apart (x = 100 px and 540 px at 100 px/m);
    # frames of the crossing events -> speed (M-CAL-D)
    v, true_x = moving_block_video(tmp_path / "m.mkv", 4.4, 100.0, FPS, 240)
    fa = int(np.argmax(true_x >= 100))
    fb = int(np.argmax(true_x >= 540))
    r = cal.speed_from_marks(fa, fb, 4.4, FPS)
    assert abs(r.speed_mps - 4.4) / 4.4 <= 0.03
    assert r.uncertainty_mps >= abs(r.speed_mps - 4.4) * 0.5  # truth lies within about the 1-frame band


def _profile(v_px, t_acc, t_run, t_dec, x0=40.0):
    """Linear acceleration to v_px, run, linear deceleration: x(t) in pixels."""
    def x(t):
        if t < t_acc:
            return x0 + 0.5 * v_px / t_acc * t * t
        a = x0 + 0.5 * v_px * t_acc
        if t < t_acc + t_run:
            return a + v_px * (t - t_acc)
        b = a + v_px * t_run
        u = min(t - t_acc - t_run, t_dec)
        return b + v_px * u - 0.5 * v_px / t_dec * u * u
    return x


def test_accel_and_stop_times(tmp_path):
    px_per_m, v_mps = 100.0, 4.4
    t_acc, t_run, t_dec = 0.25, 1.0, 0.20
    v, _ = moving_block_video(tmp_path / "a.mkv", 0, px_per_m, FPS, int(1.9 * FPS),
                              profile=_profile(v_mps * px_per_m, t_acc, t_run, t_dec, x0=40.0))
    t = cal.track_bright_blob(v, thresh=130)
    pos_m = t[:, 1] / px_per_m
    sp = cal.speed_series(t[:, 0], pos_m, FPS, win=5)
    steady = float(np.median(sp[int(0.5 * FPS):int(1.2 * FPS)]))
    assert steady == pytest.approx(4.4, rel=0.01)
    t0, t95 = cal.accel_time_95(sp, FPS, steady)
    acc = t95 - t0
    release = int((t_acc + t_run) * FPS)
    stop = cal.stop_time(sp, release, FPS, steady)
    print(f"accel to 95%: {acc:.4f} s (true {0.95 * t_acc:.4f}); stop to 5%: {stop:.4f} s (true {0.95 * t_dec:.4f})")
    assert abs(acc - 0.95 * t_acc) <= 2 / FPS
    assert abs(stop - 0.95 * t_dec) <= 2 / FPS


def _dot_video(path, fov_deg, width, height, yaw_start, yaw_end, n, fps, az=10.0):
    f = (width / 2) / math.tan(math.radians(fov_deg / 2))
    yaws = np.linspace(yaw_start, yaw_end, n)
    xs = width / 2 + f * np.tan(np.radians(az - yaws))
    yy, xx = np.mgrid[0:height, 0:width]

    def gen():
        for x in xs:
            g = 255 * np.exp(-((xx - x) ** 2 + (yy - height / 2) ** 2) / (2 * 3.0 ** 2))
            yield np.dstack([g] * 3).astype(np.uint8)
    write_video(path, gen(), fps, width, height)
    return yaws, xs


def test_fov_fit_from_video(tmp_path):
    w, h = 1280, 160
    truth = 78.0
    yaws, xs = _dot_video(tmp_path / "f.mkv", truth, w, h, -25.0, 25.0, 60, FPS)
    t = cal.track_bright_blob(tmp_path / "f.mkv", thresh=20)
    assert np.abs(t[:, 1] - xs - 0.0).max() < 1.0   # tracker accuracy (px) incl. half-pixel convention
    fit = cal.fov_from_pairs(t[::5, 1], 10.0 - yaws[::5], w)     # theta = compass azimuth of the landmark
    print(f"FOV fit: {fit.fov_deg:.3f} deg (true {truth}), rms {fit.rms_px:.3f} px")
    assert abs(fit.fov_deg - truth) <= 0.5
    assert fit.px_per_deg_center == pytest.approx(fit.focal_px * math.pi / 180)


def test_fov_fit_numeric_noise_robust():
    rng = np.random.default_rng(0)
    for truth in (60.0, 90.0, 110.0):
        w = 1920
        f = (w / 2) / math.tan(math.radians(truth / 2))
        th = np.linspace(-20, 20, 25)
        x = w / 2 + f * np.tan(np.radians(th)) + rng.normal(0, 0.7, th.size)
        fit = cal.fov_from_pairs(x, th, w)
        assert abs(fit.fov_deg - truth) / truth <= 0.01


def test_angle_helpers():
    assert cal.px_to_deg(35.0, 17.5) == 2.0
    assert cal.drop_m(0.5, 100.0) == pytest.approx(0.8727, rel=1e-3)
    assert cal.zoom_ratio(90.0, 90.0) == pytest.approx(1.0)
    assert cal.zoom_ratio(90.0, 2 * math.degrees(math.atan(0.5))) == pytest.approx(2.0)


def test_transition_time_from_horizon_video(tmp_path):
    w, h, n = 160, 200, 120
    start, dur_f = 30, 30                       # linear ramp over 30 frames = 0.25 s @120 fps

    def gen():
        for i in range(n):
            u = min(max((i - start) / dur_f, 0.0), 1.0)
            y = int(round(60 + 80 * u))
            f = np.zeros((h, w, 3), np.uint8)
            f[y:] = 200
            yield f
    v = write_video(tmp_path / "t.mkv", gen(), FPS, w, h)
    ys = np.array([float(np.argmax(f[:, w // 2] > 100)) for f in
                   (cv2.cvtColor(g, cv2.COLOR_GRAY2BGR) if False else g for _, g in
                    __import__("ref_analysis.extract_frames", fromlist=["x"]).iter_frames(v))])
    tr = cal.transition_time(ys, FPS, tol_frac=0.03)
    print(f"transition: {tr.duration_s:.4f} s (true {dur_f / FPS:.4f}), +-{tr.uncertainty_s * 1000:.2f} ms")
    assert abs(tr.duration_s - dur_f / FPS) <= 2 / FPS
    with pytest.raises(ValueError):
        cal.transition_time(np.ones(50), FPS)
