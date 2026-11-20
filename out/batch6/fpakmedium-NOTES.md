# B6-fpakmedium

2026-11-20，工作站（PI-109，非参考机）。E1-9、E1-14 的 Carentan 1/3 Medium 构建门禁通过。M1-064 仍为 partial。

- `asset-pipeline carentan-medium --incoming <cache> --out <new-dir>` 用时 703.02 s。包含 380 地形块及其碰撞、5 个已登记 CC0 模型、15 个灰盒模型和 22 个 Opus/AAC 合成音效。
- `FRONTLINE_ASSET_PACKS=<new-dir> CARGO_BUILD_JOBS=8 scripts/local-ci.sh assets` 用时 100.80 s。175 单测、11 e2e、35 包及 870 条目逐字节往返通过。首图 107,438,594 B，生产 core Brotli 1,189,937 B。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh determinism` 通过。3×18,000 tick 原生和 WASM 哈希、位置一致。`node tools/ci/check-acceptance.mjs` 通过。
- M1 缺失报告 5 real、14 placeholder、49 missing、4 internal-only。运行时素材接线、音频混音、参考机与 High/Ultra 未测。实录和视觉部分待 Fable 设计及 H-047/H-058 决定。
- 清单与记录入库。原档、二进制包和中间文件保存在任务临时目录供验证，收尾清理。
