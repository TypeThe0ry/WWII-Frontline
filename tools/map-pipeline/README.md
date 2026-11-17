# map-pipeline

Python map pipeline (ARCHITECTURE §4.4). M0 scope (M0-009): one 256 m block with procedural
terrain, a 8 km far ring, 4x4 chunks of 64 m, a validator. Must not be depended on by `crates/*`
at runtime. Needs `numpy` (+ `pytest` for tests); GDAL (`osgeo`) only for `--dem`.

```sh
export PYTHONPATH=tools/map-pipeline
python3 -m map_pipeline m0 --seed 1 --out data/maps/m0      # procedural (default)
python3 -m map_pipeline m0 --seed 1 --out data/maps/m0 \
    --dem assets/incoming/maps/carentan/<file>.tif --window E,N   # needs GDAL; E,N = SW sample point
python3 -m map_pipeline validate data/maps/m0               # exit 1 on any error
(cd tools/map-pipeline && python3 -m pytest -q)
```

Conventions: x east, z north, SW corner = (0,0); arrays row-major, row = z, column = x; u16
little-endian; `h = hmin_m + u16/65535*(hmax_m-hmin_m)`. Chunk `b_{bx}_{bz}.bin` = 65x65 u16 (rows z);
`c_{bx}_{bz}.bin` = `"FCOL" | u8 version=1 | u8 pad | u16 65 | f32 hmin | f32 hmax | u16 65x65 | u8 64x64`.
Output is byte-for-byte deterministic for a given seed/tool version. Binaries (`*.r16`, `chunks/`) are
git-ignored; `manifest.json` (tracked) holds their sizes and SHA-256.
