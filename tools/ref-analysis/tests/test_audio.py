import numpy as np
import pytest

from ref_analysis import audio_events as ae
from ref_analysis import extract_frames as ef
from synth import flash_video, gunshot_audio, write_video, write_wav

RATE = 48000


def times(rpm, n, t0=0.5):
    return t0 + np.arange(n) * 60.0 / rpm


def rpm_err(res, rpm):
    return abs(res.rpm - rpm) / rpm * 100.0


def test_600rpm_clean_within_one_percent():
    t = times(600, 31)
    r = ae.detect_events(gunshot_audio(t, 5.0, snr_db=40), RATE)
    assert r.n == 31
    print(f"600 rpm clean: {r.rpm:.3f} rpm, err {rpm_err(r, 600):.4f} %")
    assert rpm_err(r, 600) <= 1.0


@pytest.mark.parametrize("rpm", [40, 120, 450, 600, 900, 1200])
def test_rate_range_with_echo_and_noise(rpm):
    n = 31
    t = times(rpm, n)
    x = gunshot_audio(t, t[-1] + 1.0, snr_db=20, echo_delay_s=min(0.03, 0.4 * 60 / rpm), seed=rpm)
    r = ae.detect_events(x, RATE)
    assert r.n == n, (r.n, r.params)
    assert rpm_err(r, rpm) <= 1.0, (r.rpm, rpm)


def test_peak_mode_also_works():
    t = times(600, 31)
    r = ae.detect_events(gunshot_audio(t, 5.0, snr_db=30), RATE, mode="peak")
    assert r.n == 31 and rpm_err(r, 600) <= 1.0


def test_noise_robustness_down_to_10db():
    t = times(600, 31)
    errs = []
    for seed in range(5):
        r = ae.detect_events(gunshot_audio(t, 5.0, snr_db=10, seed=seed), RATE)
        assert r.n == 31, (seed, r.n)
        errs.append(rpm_err(r, 600))
    print("600 rpm at 10 dB SNR, 5 seeds: max err %.4f %%" % max(errs))
    assert max(errs) <= 1.0


def test_tick_quantised_intervals_are_averaged():
    # 30 Hz server tick: 12.857-tick interval (140 rpm) quantised to alternating 12/13 ticks (3.3 / 7)
    ticks = np.cumsum([0] + [12 if i % 7 else 13 for i in range(40)])
    t = 0.3 + ticks / 30.0
    true_rpm = 60 * (len(t) - 1) / (t[-1] - t[0])
    r = ae.detect_events(gunshot_audio(t, t[-1] + 1, snr_db=30), RATE)
    assert r.n == len(t)
    assert abs(r.rpm - true_rpm) / true_rpm <= 0.005
    assert r.intervals_s.min() < r.intervals_s.max()   # individual intervals are quantised, mean is not


def test_drop_first_shot():
    t = times(600, 21)
    x = gunshot_audio(t, 4.0, snr_db=40)
    r = ae.audio_rpm(_wav(x), drop_first=1)
    assert r.n == 20 and rpm_err(r, 600) <= 1.0


def _wav(x, _cache={}):
    import tempfile, pathlib
    d = pathlib.Path(tempfile.mkdtemp())
    return write_wav(d / "a.wav", x)


def test_too_few_events_returns_nan():
    r = ae.summarize_times(np.array([1.0]))
    assert np.isnan(r.rpm)


@pytest.mark.ffmpeg
def test_through_ffmpeg_mux_and_extract(tmp_path):
    t = times(600, 31)
    wav = write_wav(tmp_path / "in.wav", gunshot_audio(t, 5.0, snr_db=30))
    vid = write_video(tmp_path / "cap.mkv", (np.zeros((90, 160, 3), np.uint8) for _ in range(300)), 60, 160, 90,
                      codec="libx264", audio=wav)
    out = ef.extract_audio(vid, tmp_path / "out.wav")
    r = ae.audio_rpm(out)
    print(f"600 rpm through mkv(pcm) -> wav: {r.rpm:.3f} rpm, err {rpm_err(r, 600):.4f} %")
    assert r.n == 31 and rpm_err(r, 600) <= 1.0


@pytest.mark.ffmpeg
def test_cross_validation_audio_vs_video_flash_within_2_percent(tmp_path):
    # 3.3 step 6: the same burst measured through the soundtrack and through the muzzle flashes in the video
    from ref_analysis import frame_events as fe
    fps = 120
    t = times(600, 31, t0=0.5)
    onsets = [int(round(x * fps)) for x in t]
    nfr = int(5.0 * fps)
    v = flash_video(tmp_path / "v.mkv", onsets, nfr, fps)
    ser = fe.luminance_series(v, roi=(60, 40, 130, 90))
    fr = fe.rpm_from_frames(fe.detect_onsets(ser), fps)
    a = ae.detect_events(gunshot_audio(t, 5.0, snr_db=30), RATE)
    assert abs(fr.rpm - a.rpm) / a.rpm <= 0.02
