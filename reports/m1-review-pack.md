# 检查点 1 审阅材料（MILESTONES §2.10，M1-072）

1. **测试局记录**：无真人局；见 `m1-playtest-log.md`。headless 20+12 bot 300 s 全通过。
2. **5 兵种与 Sherman 假设评估**：数值均为 `initial`；批 M1 录像为 0，手感数据无对标（M1-071 blocked）。
3. **带宽/tick 偏差**：实测远低于预算（tick P99 3.54 ms vs 28 ms；下行 P99 26.6 vs 400 kbps），但仅无损同机；netem 与 30 min WT 长跑未做，偏差结论不可靠。
4. **素材到位与占位**：见 `m1-placeholders.md` 与 `docs/ASSET_GAPS.md` §2/§3.7；M1 72 个需求 ID（来源级 5 real / 7 placeholder / 56 missing / 4 internal-only，台账 `ledger-check` 242 行 OK 232）已按零采购默认假设完成 72/72 逐 ID 裁决（5 `real` 免费 CC0 采用、36 免费候选采用+程序化占位、15 程序化占位+真素材推迟到 M2、16 范围外推迟到 M2，含 GAP-V01/V02/V03/V06 默认决定）；人工购买/外包/下载项留 `docs/NEEDS_HUMAN.md` H-047/H-058/H-099/H-100 待裁决（PI-108、PI-B6-gapsdecide-1）。
5. **批 M1 测量偏差**：无数据（blocked，等 H-011/H-025/H-026 录像）。
6. **任务粒度/返工率**：M1 共 72 个任务行，大部分以 "a" 部分切片交付，余项拆成 *b/*c；返工主要来自金哈希重定标（PI-043）与 ref 规格与 GAMEDESIGN 冲突（PI-106）。
