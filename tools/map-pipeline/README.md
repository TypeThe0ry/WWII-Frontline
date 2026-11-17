# map-pipeline

Python + GDAL map pipeline (DEM -> heightmap, splat, vegetation, chunks, collision, navmesh).
Skeleton only; filled in by M0-009. Reads `/data/maps/*` and `/assets/incoming`, writes to
`/assets/processed`. Must not be depended on by `crates/*` at runtime (ARCHITECTURE §4.4).
