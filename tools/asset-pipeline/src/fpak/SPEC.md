# `.fpak` container, version 1 (M1-003, D-PAK / R11)

**Honest statement.** `.fpak` exists so that processed assets are not shipped as standalone `.glb` / `.ktx2` / `.ogg` files (a
compliance measure, "non-standalone distribution"). It is **not** DRM, encryption or obfuscation. Anyone can fetch a `.fpak` and
parse it. Do not describe it as protection anywhere.

All integers little-endian. Entry payloads are 16-byte aligned (zero padding). Reference code: `format.ts`, `writer.ts`, `reader.ts`.

## File layout

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | magic `"FPAK"` |
| 4 | 2 | `version` (u16) = 1 |
| 6 | 2 | flags (u16), 0 |
| 8 | 4 | `entry_count` (u32) |
| 12 | 4 | reserved |
| 16 | 8 | `index_offset` (u64) |
| 24 | 8 | reserved |
| 32 | … | entry payloads, each at a multiple of 16 |
| `index_offset` | `index_bytes` | index |
| end − 32 | 32 | footer |

## Index (one record per entry, in write order)

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | `id_len` (u16, ≤ 255) |
| 2 | 1 | `kind` (0 mesh, 1 texture, 2 audio, 3 meta) |
| 3 | 1 | reserved |
| 4 | 2 | `codec` (u16, see below) |
| 6 | 8 | `offset` (u64, absolute) |
| 14 | 4 | `bytes` (u32, payload length without padding) |
| 18 | 8 | `sha8` = first 8 bytes of sha256(payload) |
| 26 | `id_len` | `logical_id`, UTF-8 |

`logical_id` matches `^[a-z][a-z0-9_]*(@[a-z0-9_]+)?$`: logical ids only, never source paths, ledger ids or vendor names.
Examples: `wpn_us_m1_garand@lod0_fp`, `veh_us_m4a1_sherman@collision`, `prop_crate@tex_albedo`, `prop_crate@meta`.

## Footer (fixed 32 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | `index_offset` (u64) |
| 8 | 4 | `index_bytes` (u32) |
| 12 | 4 | `entry_count` (u32) |
| 16 | 4 | `crc32` of the index bytes (IEEE) |
| 20 | 2 | `version` (u16) |
| 22 | 6 | reserved |
| 28 | 4 | end magic `"KAPF"` |

Reading: `size()` → read the last 32 bytes → read `[index_offset, +index_bytes)` (one Range request) → read any entry by
`offset`/`bytes`. Opening costs footer + index bytes only. The version lives in the footer so that opening never needs the header.

## Codecs

| codec | meaning |
|---|---|
| 0 | raw bytes |
| 1 | mesh entry layout v1 (below) |
| 2 | texture entry layout v1 (below) |
| 3 | audio (reserved for M1-004) |
| 4 | UTF-8 JSON (`meta` entries) |

## Mesh entry layout v1 (`entry-mesh.ts`)

Header: `u8 layout_version=1 | u8 flags | u16 prim_count | u16 material_count | u16 reserved | f32 bbox_min[3] | f32 bbox_max[3]`.
Then `material_count` records: `f32 base_color[4] | f32 metallic | f32 roughness | f32 emissive[3]`, four texture refs
(`u8 len | utf8 logical id`, empty = none; slots: base_color, normal, orm, emissive), padded to 4 bytes.
Then `prim_count` primitives: `u16 material_index (0xffff none) | u8 attr_count | u8 index_size (2|4) | u32 vertex_count |
u32 index_count | u32 vbuf_bytes | u32 ibuf_bytes | f32 matrix[16] (column-major, applied to POSITION)`, `attr_count` ×
`u8 semantic | u8 component_type | u8 elements | u8 normalized`, the meshopt vertex buffer (interleaved, each attribute padded to
4 bytes), padding to 4, the meshopt index buffer, padding to 4. Semantic codes: POSITION 0, NORMAL 1, TANGENT 2, TEXCOORD_0 3,
TEXCOORD_1 4, COLOR_0 5, JOINTS_0 6, WEIGHTS_0 7. Component type codes: i8 0, u8 1, i16 2, u16 3, u32 4, f32 5.
Skinned / animated meshes are not representable yet (M1-045 extends the layout with a new `layout_version`).

## Texture entry layout v1 (`entry-texture.ts`)

KTX2 payload without the standard container header, smallest mip first.
`u8 layout_version=1 | u8 supercompression | u8 level_count | u8 channels | u32 vk_format | u32 type_size | u32 width | u32 height |
u32 depth | u32 layer_count | u32 face_count | u32 dfd_offset | u32 dfd_bytes | u32 sgd_offset | u32 sgd_bytes | u16 layout_len |
layout (utf8, e.g. "RRRG" for two-channel normals)`, then `level_count` × `u32 data_offset | u32 bytes | u32 uncompressed_bytes`
(in KTX2 level order, level 0 largest; offsets relative to the data area), the DFD block, the supercompression global data,
and the data area (16-byte aligned, smallest mip first).

## Versioning

`version` is bumped for any change to the file layout, the index record or the footer; a layout version inside an entry codec
is bumped for changes to that entry layout, and gets a new codec number if old readers must reject it. A reader accepts exactly
the versions it knows and fails loudly otherwise (`FpakFormatError`). Every bump must update this file and be logged in
`docs/PLAN_ISSUES.md`.

## Publishing

Packs are written to `{out}/a/{sha256[0:2]}/{sha256[2:16]}.fpak` (content addressed; the published URL is
`/a/{sha256[0:2]}/{sha256[2:16]}.fpak`) together with `{out}/packs.json` (path, sha256, bytes, group, per-entry
`offset/bytes/codec/sha8`) which M1-005 turns into the manifest. `assert-no-standalone` fails when the output directory holds
`.glb .gltf .ktx2 .ogg .opus .wav .png .jpg`.
