# B6-assetlod · M1-002（2026-11-20）

背景法线启用 UASTC level 1 + RDO λ=2，保留 1024² 和 11 级 mip。编码器直接读取 RGBA。网格尺寸重排在 `.fpak` 索引字节往返测试中失败，已撤回。WaterBottle 真实 CC0 验收源已登记 LICENSES.md，原档仅暂存构建机。UI 视觉待 Fable 设计。

| 工作站检查（PI-109，非参考机） | 结果 |
|---|---|
| `pnpm -C tools/asset-pipeline test` | 174/174 |
| `pnpm -C tools/asset-pipeline test:e2e` | 11/11，B 双跑 6/6 文件哈希相同 |
| `check-size-ratio --max 0.25` | 同输入旧 31.48%，新 24.35%，全流程 19.54 s。A 14.69% |
| `CARGO_BUILD_JOBS=8 scripts/local-ci.sh assets data`（分组运行） | 通过，含 lint、typecheck、build、素材闸门与 78 个数据文件校验。此检出缺 client/dist，core 体积未测 |
| `CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism wasm` | 通过，3×18,000 tick 跨端哈希一致 |

法线角误差 P95 从 0.904° 升到 3.219°，hero 逐图覆盖测试通过。参考机耗时与 Fable 视觉评价未测。真实 WaterBottle 为 19.35%，Kenney bridge 和木箱分别为 43.91% / 50.81%。额外 figurine 模型索引编码断言待查（PI-B6-assetlod-2）。M1-002 的 8 项验收已满足。完整数字见 `reports/m1-002-lod.md`，决定见 PI-B6-assetlod-1。
