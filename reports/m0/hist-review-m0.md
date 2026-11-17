# M0-031 史实参数复核记录（Kar98k、M1 Garand）

核实日期：2026-11-17。只记录数值事实与定位信息，不含来源原文。结构化出处在 `data/weapons/*.json` 的 `historical._source`。

来源缩写：
- `TM9-1275`：TM 9-1275《U.S. Rifle, Cal. .30, M1》转写（biggerhammer.net，Section 1 Introduction，par. 1-5）。战时维修手册，页面无印刷页码，定位用段落号。
- `FM23-5`：FM 23-5《U.S. Rifle, Caliber .30, M1》转写（biggerhammer.net，Ch. 1，par. 3-4）。页面文字提及核战，说明是战后版，页面未写版次年份；只作交叉参考。
- `TME30-451`：TM-E 30-451《Handbook on German Military Forces》（1945-03-15，Lone Sentry 转录，Ch. VII Sec. II par. 5a）。战时情报手册，数值为约数。

可信度：A = 官方手册直接给出该数；B = 手册给出但为战后版或约数；C = 仅二手来源（登记未核实）。

| # | 武器 | 字段 | 原值 | 复核值 | 出处 | 可信度 | 状态 |
|---|---|---|---|---|---|---|---|
| 1 | M1 | caliber | 7.62×63 mm | 7.62×63 mm（不变） | TM9-1275 par.5a 只写 .30 cal；公制写法为换算 | B | 已核实 |
| 2 | M1 | cartridge | .30-06 Springfield | 不变 | TM9-1275 par.5a：Cartridge, ball, Cal. .30, M2 | B | 已核实 |
| 3 | M1 | magazine_capacity | 8 | 8（不变） | FM23-5 Ch.1 par.3（clip 最多 8 发） | B | 已核实 |
| 4 | M1 | magazine_type | en_bloc_clip | 不变 | FM23-5 Ch.1 par.4（en-block clip）；TM9-1275 只写 Clip | B | 已核实 |
| 5 | M1 | muzzle_velocity_mps | 853 | 853（不变） | FM23-5 Ch.1 par.4（M2 弹 853 m/s = 2,800 ft/s） | B | 已核实 |
| 6 | M1 | rate_of_fire_rpm_practical | 40 | **24** | FM23-5 Ch.1 par.4（训练有素射手 16-24 发/分瞄准射击；无规定最大值） | B | 已核实，改值（差 40%） |
| 7 | M1 | mass_kg | 4.3 | 4.3（不变） | TM9-1275 par.5a（9.5 lb = 4.309 kg）；FM23-5 par.4 同 | A | 已核实 |
| 8 | M1 | effective_range_m | （无） | **460**（新增） | FM23-5 Ch.1 par.4（最大有效射程 460 m） | B | 已核实 |
| 9 | Kar | caliber | 7.92×57 mm | 不变 | TME30-451 par.5a(2)（7.92 mm / 0.312 in）；57 mm 靠弹种名 | B | 已核实 |
| 10 | Kar | cartridge | 7.92×57 mm Mauser (s.S.) | 不变 | TME30-451 par.5a(3)（标准 7.92 mm 无缘弹）；s.S. 仅二手 | B | 已核实（s.S. 后缀二手） |
| 11 | Kar | magazine_capacity | 5 | 5（不变） | TME30-451 par.5a(2)（5-round clip） | B | 已核实 |
| 12 | Kar | magazine_type | stripper_clip_internal_box | 不变 | TME30-451 只写 clip；内置弹仓为二手（Wikipedia） | B | 已核实（部分二手） |
| 13 | Kar | muzzle_velocity_mps | 760 | 760（保留） | 无一手来源。二手：Wikipedia 760（无引文）、quartermastersection 755、WWII Online wiki 745 | C | **未核实** |
| 14 | Kar | rate_of_fire_rpm_practical | 15 | 15（保留） | 无一手来源。二手：dday-overlord.com 15（无出处） | C | **未核实** |
| 15 | Kar | mass_kg | 3.9 | 3.9（保留） | TME30-451 par.5a(2)：约 9 lb（约 4.08 kg，三型共用，约数），差 4.4% 未超 5% 阈值；二手 3.7-4.1 kg / 3.92 kg | B | 已核实（约数） |

计数：共 15 个 historical 字段；已核实 13，未核实 2（登记于 Kar98k `_meta.unverified[]`），13 + 2 = 15。

## 说明与争议
- 字段 6：GAMEDESIGN §6.2 的 40 rpm 出自作者记忆。FM 23-5 给的是瞄准射击速率范围 16-24。二手文献常见的 40-50 发/分（不瞄准速射）本次未能核实。本任务取 24（手册范围上限）。`gameplay.fire.cap_rpm=120` 是节奏上限，不改，偏离原因记在 `_meta.deviation_reason`。
- 字段 8：Lone Sentry 的 M1 页面给 440 yd（约 402 m），为二手；采用 FM 23-5 的 460 m。
- Kar98k 的有效射程 TM-E 30-451 写 800 yd（约 732 m），Wikipedia 写铁瞄 500 m：两者冲突，本任务未写入 `effective_range_m`（该字段可选），留待 M1-070 用 Hogg 或博物馆馆藏裁定。
- 未找到：Royal Armouries / Smithsonian / IWM 的 Kar98k 或 M1 馆藏条目、HDv/Merkblatt 德军条令、Hogg 书目。均未登记为来源。
- Kar98k 枪口初速 760 与 755 的差别小于 1%，不影响 `gameplay.ballistics`（保持 760，未改）。
- 全长字段：schema 无 `length_mm`，本任务未新增（记录：M1 43.6 in = 1,107 mm，TM9-1275 par.5a；Kar98k 43.5 in，TME30-451 par.5a(1)）。
