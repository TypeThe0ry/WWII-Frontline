| machine | renderer | viewport | cam | runs | frame interval P50 (ms) | fps (mean) | P50 range | CPU P50/P99 (ms) | draw calls | triangles | veg instances | JS heap (MB) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| xeon4-swiftshader | webgl2 | 640x360 | 0 | 3 | 588.3 | 1.45 | 2.9 % | 1.30 / 3.60 | 50 | 450341 | 8130 | 58.4 |
| xeon4-swiftshader | webgl2 | 640x360 | 1 | 3 | 311.7 | 3.06 | 2.5 % | 1.20 / 4.20 | 33 | 9884 | 4942 | 58.4 |
| xeon4-swiftshader | webgl2 | 640x360 | 2 | 3 | 271.1 | 3.80 | 7.9 % | 1.10 / 3.70 | 13 | 318492 | 3197 | 58.5 |

CPU time per frame stage, P50 / P99 (ms), median of the runs:

| machine | renderer | cam | babylon_active_meshes_eval | babylon_render_pass | game_update | overlay | scene_render | terrain_update | vegetation_update |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| xeon4-swiftshader | webgl2 | 0 | 0.20 / 0.40 | 0.40 / 2.20 | 0.00 / 0.10 | 0.20 / 0.50 | 0.70 / 3.10 | 0.30 / 0.60 | 0.10 / 0.20 |
| xeon4-swiftshader | webgl2 | 1 | 0.20 / 1.30 | 0.30 / 3.20 | 0.00 / 0.10 | 0.20 / 1.40 | 0.60 / 3.50 | 0.30 / 1.10 | 0.10 / 1.10 |
| xeon4-swiftshader | webgl2 | 2 | 0.20 / 0.40 | 0.20 / 1.50 | 0.00 / 0.10 | 0.20 / 0.70 | 0.50 / 2.00 | 0.30 / 1.00 | 0.10 / 2.50 |
