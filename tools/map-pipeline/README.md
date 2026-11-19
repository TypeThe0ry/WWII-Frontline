# map-pipeline

Python map pipeline (ARCHITECTURE §4.4) for M0 procedural terrain and M1 Carentan 1/3 slice
(M1-030…M1-036). Must not be depended on by `crates/*` at runtime.

## Dependencies & No-GDAL Path

- **Base (GDAL-free)**: Python 3 + `numpy` + `jsonschema` (+ `pytest` for tests).
- **GDAL (`osgeo.gdal`, e.g. `apt install python3-gdal`)**: **ONLY** required by two raw-GeoTIFF raster
  stages (`m0 --dem ...` and `dem`). All other stages run completely without GDAL using pure NumPy
  and committed/processed map files.
- **Node (`tools/nav-bake`)**: Only required by `nav` and `nav-query` (uses `recast-navigation` WASM).

| Command | Stage | Needs GDAL? | Needs Node? | Notes |
| --- | --- | --- | --- | --- |
| `m0 --seed 1 --out data/maps/m0` | M0-009 | No | No | Procedural 256 m block |
| `m0 --dem <tif> --window E,N` | M0-009 | **Yes** | No | Fails cleanly with `GdalMissing` (exit 1) if `osgeo` absent |
| `carentan-meta` / `validate-sources` | M1-030 | No | No | Anchors, CRS, `LICENSES.md` §3 gate |
| `dem --incoming ... --out ...` | M1-031 | **Yes** | No | Missing raster → exit 3; unverified ledger → exit 4; missing GDAL → exit 1 |
| `dem-qa data/maps/carentan` | M1-031 | No | No | Water-mask datum & canal-bed elevation check on committed `.r16` |
| `osm` / `validate-vectors` / `check-roads` | M1-032 | No | No | Parses committed `overpass_carentan.json.gz` (pure Python) |
| `edits-1944` | M1-033 | No | No | Pure NumPy polygon rasterisation + smoothstep feather; missing human inputs → exit 4 |
| `zones-m1` / `validate --slice m1` | M1-034 | No | No | Zones, spawns, landmarks, building annotation, 16-rule validator |
| `chunks-m1` / `pack-chunks` | M1-035 | No | Optional | 64 m `FCHK`/`FCOL` bake + `FPAK` v1 packing & manifest fragment |
| `nav --slice m1` / `nav-query` | M1-036 | No | **Yes** | `tools/nav-bake` (`recast-navigation` 0.44.0) |

## Quick Commands

```sh
export PYTHONPATH=tools/map-pipeline
python3 -m map_pipeline m0 --seed 1 --out data/maps/m0
python3 -m map_pipeline validate data/maps/m0
python3 -m map_pipeline dem-qa data/maps/carentan --report reports/m1-dem-water-check.json
python3 -m map_pipeline osm
python3 -m map_pipeline validate-vectors data/maps/carentan
python3 -m map_pipeline check-roads data/maps/carentan
python3 -m map_pipeline zones-m1
python3 -m map_pipeline validate data/maps/carentan --slice m1
python3 -m map_pipeline chunks-m1
python3 -m map_pipeline pack-chunks
python3 -m map_pipeline nav --slice m1
python3 -m map_pipeline nav-query --from 476,1600 --to 1273,911
(cd tools/map-pipeline && python3 -m pytest -q)
```

Conventions: x east, z north, SW corner = (0,0); arrays row-major, row = z, column = x; u16
little-endian; `h = hmin_m + u16/65535*(hmax_m-hmin_m)`. Chunk `b_{bx}_{bz}.bin` = 65x65 u16 (rows z);
`c_{bx}_{bz}.bin` = `"FCOL" | u8 version=1 | u8 pad | u16 65 | f32 hmin | f32 hmax | u16 65x65 | u8 64x64`.
Output is byte-for-byte deterministic for identical inputs/seed/tool version.
