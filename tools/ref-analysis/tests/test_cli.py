import json

import numpy as np
import pytest

from ref_analysis import cli
from synth import flash_video, gunshot_audio, write_wav


def run(capsys, *argv):
    code = cli.main(list(argv))
    out = capsys.readouterr().out
    return code, out


def test_stats_and_speed_and_fov_commands(capsys):
    code, out = run(capsys, "stats", "600,604,598,602,601", "--frame-unc", "0.1")
    j = json.loads(out)
    assert code == 0 and j["median"] == 601 and j["confidence"] == "B" and j["uncertainty"] == 3.0
    code, out = run(capsys, "speed", "--frame-a", "10", "--frame-b", "2740", "--distance", "100", "--fps", "120")
    assert abs(json.loads(out)["speed_mps"] - 100 / (2730 / 120)) < 1e-9
    import math
    w = 1920
    f = (w / 2) / math.tan(math.radians(40))
    th = np.linspace(-15, 15, 11)
    x = w / 2 + f * np.tan(np.radians(th))
    code, out = run(capsys, "fov", "--x", ",".join(f"{v:.3f}" for v in x), f"--theta={','.join(map(str, th))}", "--width", str(w))
    assert abs(json.loads(out)["fov_deg"] - 80) < 0.05


@pytest.mark.ffmpeg
def test_probe_audio_and_frame_rpm_commands(capsys, tmp_path, monkeypatch):
    monkeypatch.setenv("REF_CAPTURE_DIR", str(tmp_path))
    onsets = [10 + 12 * k for k in range(25)]
    flash_video(tmp_path / "f.mkv", onsets, 330, 120)
    code, out = run(capsys, "probe", "f.mkv")
    assert code == 0 and json.loads(out)["cfr"] is True
    code, out = run(capsys, "frame-rpm", "f.mkv", "--roi", "60:40:130:90")
    assert abs(json.loads(out)["rpm"] - 600) < 1e-6
    t = 0.4 + np.arange(25) * 0.1
    write_wav(tmp_path / "a.wav", gunshot_audio(t, 3.5, snr_db=30))
    code, out = run(capsys, "audio-rpm", "a.wav")
    assert abs(json.loads(out)["rpm"] - 600) < 6


def test_status_gate_fails_while_batch_incomplete(capsys, tmp_path, monkeypatch):
    from ref_analysis import record
    record.write_summary(tmp_path / "summary.csv", [])
    monkeypatch.setenv("REF_MEASUREMENTS_DIR", str(tmp_path))
    code, out = run(capsys, "status")
    assert code == 1 and "74 rows" in out and "74 missing" in out
    code, out = run(capsys, "validate")
    assert code == 0 and "25 columns" in out


def test_status_gate_passes_when_every_row_is_measured_with_a_record(capsys, tmp_path, monkeypatch):
    from ref_analysis import record, stats
    from ref_analysis.items import all_rows
    monkeypatch.setenv("REF_MEASUREMENTS_DIR", str(tmp_path))
    record.write_summary(tmp_path / "summary.csv", [])
    st = stats.summarize([1.0, 1.0, 1.0, 1.0, 1.0])
    for it, rid, sid in all_rows():
        row = record.row_from_stats(it, rid, sid, st, fps=120, method=it.method, scene=it.scene,
                                    ref_version="synthetic", date="2026-11-17", confidence="B")
        row["pass"] = "yes"
        record.upsert_summary_row(tmp_path / "summary.csv", row)
        (tmp_path / f"{rid}.md").write_text("synthetic", encoding="utf-8")
    code, out = run(capsys, "status")
    assert code == 0 and "0 missing" in out
    # one infeasible row without pass is fine; one failing row blocks the gate
    rows = record.read_summary(tmp_path / "summary.csv")
    rows[0]["pass"] = "no"
    record.write_summary(tmp_path / "summary.csv", rows)
    code, out = run(capsys, "status")
    assert code == 1 and "1 measured but not pass" in out
