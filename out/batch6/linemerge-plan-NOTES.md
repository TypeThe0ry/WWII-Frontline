# B6-linemerge-plan — 2026-11-20

- **已做**：在 `b6/linemerge-plan` 独立 worktree 固定 MS `0e5b41f`、P `196a3de`；交付 `docs/LINE_RECONCILE.md`，追加 PI、状态、M1 报告及 H-017/021/056 进展。确认本次是 MS v4 对 P v10；v8 是 P 历史检查点。按 MS 网关与验收基线分阶段合流是建议，不是本次决策实施。
- **试算**：`git merge-base --all` → `873e2d7`；`git rev-list --left-right --count MS...P` → 163 / 211；`git merge-tree --write-tree --name-only --messages -z MS P` → 退出 1（预期）、131 路径（130 文本 412 冲突块 + 1 二进制），39 个自动合并的双改路径。未改任何产品代码、未合并两条产品线。
- **自检（PI-109 工作站，非参考机）**：`pnpm install --frozen-lockfile --offline`；`CARGO_BUILD_JOBS=8 scripts/local-ci.sh determinism` 通过（native 4/4、漂移统计 5/5、地形一致性 1/1）；复用轨迹 `node tools/determinism/compare.mjs --strict target/determinism <临时目录>` 通过（3×18,000 tick 哈希/位置一致，最大位置误差 0 m）。恢复自检生成的已跟踪 M0 summary，无基线变更。未使用 GitHub Actions 或 GPU。
- **未做 / 遗留**：P 未在本批复跑；合流候选、旧版/新版实物互连、参考机、受信任证书、真人及录像均未测。未引入素材；待 Fable 设计。后续先定 wire、票据、模式/地图 ID 和数据契约，再按文档清单验证。
- **落地范围**：仅此文档及记录通过 `land-ms.sh linemerge-plan` 落地 MS；不落 `work/playable`。
- **落地复核**：MS 前移至 `f4f18a7` 时仅有 E1-10 文档变更；复跑 `merge-tree`，对同一 P 得 164 / 211 个独有提交、完全相同的 131 条冲突路径。文档 rebase 的四个冲突块已保留双方内容。
