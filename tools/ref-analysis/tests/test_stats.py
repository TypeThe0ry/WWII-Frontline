import math

import pytest

from ref_analysis import stats


def test_summarize_basic_and_uncertainty_rule():
    s = stats.summarize([600, 604, 596, 602, 598], frame_unc=0.5)
    assert s.n == 5 and s.median == 600 and s.min == 596 and s.max == 604
    assert s.range == 8 and s.uncertainty == 4.0  # max(range/2, frame)
    s2 = stats.summarize([1.0, 1.0, 1.0], frame_unc=0.0083)
    assert s2.uncertainty == pytest.approx(0.0083)  # one frame dominates a zero range
    assert stats.summarize([5.0]).range_pct == 0.0


def test_confidence_boundaries_n_and_range():
    # A: n >= 10 and range <= 5 %
    assert stats.confidence(10, 5.0) == "A"
    assert stats.confidence(10, 5.0001) == "B"
    assert stats.confidence(9, 1.0) == "B"          # n = 9 is not A
    # B: n >= 5 and range <= 20 %
    assert stats.confidence(5, 20.0) == "B"
    assert stats.confidence(5, 20.0001) == "C"
    assert stats.confidence(4, 1.0) == "C"           # n < 5
    assert stats.confidence(30, 25.0) == "C"
    # modifiers
    assert stats.confidence(10, 1.0, public_conflict=True) == "B"
    assert stats.confidence(5, 10.0, public_conflict=True) == "C"
    assert stats.confidence(10, 1.0, cross_validated=False) == "C"
    assert stats.confidence(10, 1.0, cross_validated=True) == "A"
    assert stats.confidence(0, 0.0, infeasible=True) == "D"


def test_range_pct_from_values():
    s = stats.summarize([100, 110, 105, 100, 95, 100, 100, 100, 100, 100])
    assert s.n == 10 and s.range_pct == pytest.approx(15.0)
    assert stats.confidence(s.n, s.range_pct) == "B"
    assert stats.summarize([0.0, 0.0]).range_pct == 0.0


def test_speed_error_propagation():
    # d = 100 +- 1 m, t = 22.7 +- 0.0083 s
    u = stats.speed_unc(100.0, 1.0, 22.7, 0.0083)
    v = 100 / 22.7
    assert u / v == pytest.approx(math.hypot(0.01, 0.0083 / 22.7))


@pytest.mark.parametrize("kind,target,tol", [
    ("timing", 0.2, 1 / 60),            # < 0.3 s: one 60 fps frame
    ("timing", 0.3, 1 / 60),            # 5 % = 15 ms < 16.7 ms
    ("timing", 1.0, 0.05),              # >= 1 s: 5 %
    ("speed", 4.4, 0.132),
    ("distance", 10.0, 1.0),            # +-1 m dominates
    ("distance", 100.0, 5.0),
    ("angle", 1.0, 0.1),
    ("angle", 10.0, 0.5),
    ("stat", 3.0, 0.3),
    ("count", 2.0, 0.0),
])
def test_tolerance_table_7_3(kind, target, tol):
    assert stats.tolerance_abs(kind, target) == pytest.approx(tol)


def test_reference_uncertainty_relaxes_tolerance():
    assert stats.tolerance_abs("speed", 4.4, ref_unc=0.5) == 0.5


def test_compare_pass_fail_and_exact_counts():
    assert stats.compare("speed", 4.45, 4.4).passed
    assert not stats.compare("speed", 4.6, 4.4).passed
    assert stats.compare("count", 2, 2).passed
    assert not stats.compare("count", 3, 2).passed
    c = stats.compare("timing", 0.5, 0.5)
    assert c.error == 0 and c.error_pct == 0 and c.passed
    with pytest.raises(ValueError):
        stats.tolerance_abs("nonsense", 1)
