"""WGS84 <-> UTM (Krüger series, sub-millimetre inside a zone) and the map-local frame.

M1-030 (MAPS §1.3): the working CRS is ETRS89 / UTM 30N (EPSG:25830), equal to WGS84 / UTM 30N
(EPSG:32630) at this project's precision. Pure Python so the tool needs no ``pyproj``; the tests
cross-check against ``pyproj`` when it is installed.
"""
from __future__ import annotations

import math

A = 6378137.0
F = 1 / 298.257223563
K0 = 0.9996
E0 = 500000.0

_N = F / (2 - F)
_RECT = A / (1 + _N) * (1 + _N**2 / 4 + _N**4 / 64)
_ALPHA = (
    _N / 2 - 2 * _N**2 / 3 + 5 * _N**3 / 16,
    13 * _N**2 / 48 - 3 * _N**3 / 5,
    61 * _N**3 / 240,
)
_BETA = (
    _N / 2 - 2 * _N**2 / 3 + 37 * _N**3 / 96,
    _N**2 / 48 + _N**3 / 15,
    17 * _N**3 / 480,
)
_DELTA = (
    2 * _N - 2 * _N**2 / 3 - 2 * _N**3,
    7 * _N**2 / 3 - 8 * _N**3 / 5,
    56 * _N**3 / 15,
)


def central_meridian(zone: int) -> float:
    return math.radians(zone * 6 - 183)


def to_utm(lat_deg: float, lon_deg: float, zone: int = 30) -> tuple[float, float]:
    """(E, N) in metres, northern hemisphere."""
    phi = math.radians(lat_deg)
    dl = math.radians(lon_deg) - central_meridian(zone)
    e = 2 * math.sqrt(_N) / (1 + _N)
    t = math.sinh(math.atanh(math.sin(phi)) - e * math.atanh(e * math.sin(phi)))
    xi_p = math.atan2(t, math.cos(dl))
    eta_p = math.atanh(math.sin(dl) / math.sqrt(1 + t * t))
    xi = xi_p + sum(a * math.sin(2 * j * xi_p) * math.cosh(2 * j * eta_p) for j, a in enumerate(_ALPHA, 1))
    eta = eta_p + sum(a * math.cos(2 * j * xi_p) * math.sinh(2 * j * eta_p) for j, a in enumerate(_ALPHA, 1))
    return E0 + K0 * _RECT * eta, K0 * _RECT * xi


def from_utm(easting: float, northing: float, zone: int = 30) -> tuple[float, float]:
    """(lat, lon) in degrees."""
    xi = northing / (K0 * _RECT)
    eta = (easting - E0) / (K0 * _RECT)
    xi_p = xi - sum(b * math.sin(2 * j * xi) * math.cosh(2 * j * eta) for j, b in enumerate(_BETA, 1))
    eta_p = eta - sum(b * math.cos(2 * j * xi) * math.sinh(2 * j * eta) for j, b in enumerate(_BETA, 1))
    chi = math.asin(math.sin(xi_p) / math.cosh(eta_p))
    phi = chi + sum(d * math.sin(2 * j * chi) for j, d in enumerate(_DELTA, 1))
    lam = central_meridian(zone) + math.atan2(math.sinh(eta_p), math.cos(xi_p))
    return math.degrees(phi), math.degrees(lam)


def to_local(e: float, n: float, origin_e: float, origin_n: float) -> tuple[float, float]:
    """Map-local metres: origin at the SW corner, x east, y north."""
    return e - origin_e, n - origin_n


def from_local(x: float, y: float, origin_e: float, origin_n: float) -> tuple[float, float]:
    return x + origin_e, y + origin_n
