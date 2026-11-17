import numpy as np
import pytest

from ref_analysis import common, extract_frames as ef, frame_events as fe
from synth import flash_video, write_video

pytestmark = pytest.mark.ffmpeg


def test_probe_cfr_and_frame_count(tmp_path):
    v = flash_video(tmp_path / "f.mkv", [10, 22], 60, 120)
    i = common.probe(v)
    assert i.is_cfr and float(i.fps) == 120 and (i.width, i.height) == (320, 180)
    assert i.frame_unc_s == pytest.approx(1 / 120)
    assert ef.count_frames(v) == 60            # passthrough: no duplicated / dropped frames
    n = ef.extract_pngs(v, tmp_path / "frames", crop=(100, 60, 10, 10), t0=0.0, t1=0.25)
    assert n == 30


def test_frame_time_conversion():
    assert common.frame_to_time(240, 120) == 2.0
    dt, unc = common.interval_s(10, 22, 120)
    assert dt == pytest.approx(0.1) and unc == pytest.approx(1 / 120)


def test_flash_onsets_exact_and_rpm_600_at_120fps(tmp_path):
    onsets = [20 + 12 * k for k in range(31)]          # 12 frames @120 fps = 0.1 s = 600 rpm
    v = flash_video(tmp_path / "f.mkv", onsets, 420, 120)
    s = fe.luminance_series(v, roi=(60, 40, 130, 90))
    got = fe.detect_onsets(s)
    assert got == onsets
    r = fe.rpm_from_frames(got, 120)
    assert r.rpm == pytest.approx(600.0)
    assert r.uncertainty_rpm == pytest.approx(600.0 / (12 * 30))


def test_non_integer_interval_long_burst_average_error_below_one_percent(tmp_path):
    fps = 144                                              # 600 rpm = 14.4 frames; onsets quantised
    onsets = [int(np.floor(15 + 14.4 * k)) for k in range(31)]
    v = flash_video(tmp_path / "f.mkv", onsets, 500, fps)
    got = fe.detect_onsets(fe.luminance_series(v, roi=(60, 40, 130, 90)))
    assert got == onsets
    r = fe.rpm_from_frames(got, fps)
    err = abs(r.rpm - 600) / 600 * 100
    print(f"600 rpm @144 fps from flash frames: {r.rpm:.3f} rpm, err {err:.4f} %")
    assert err <= 1.0
    assert abs(r.rpm - 600) <= r.uncertainty_rpm + 1e-9   # error inside the stated +-1 frame bound


def test_lossy_encoding_still_detects_flashes(tmp_path):
    onsets = [10 + 15 * k for k in range(20)]
    v = flash_video(tmp_path / "f.mp4", onsets, 330, 60, codec="libx264")
    got = fe.detect_onsets(fe.luminance_series(v, roi=(60, 40, 130, 90)))
    assert got == onsets


def test_event_delta():
    dt, unc = fe.event_delta(100, 190, 120)
    assert dt == pytest.approx(0.75) and unc == pytest.approx(1 / 120)


def test_no_flash_gives_no_onsets(tmp_path):
    v = flash_video(tmp_path / "f.mkv", [], 40, 60)
    assert fe.detect_onsets(fe.luminance_series(v)) == []
