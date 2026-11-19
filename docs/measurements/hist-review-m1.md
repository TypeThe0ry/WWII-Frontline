# M1 史实参数复核记录（M1-070）

**M1-070b · 2026-11-19：**按 `data/` 逐项核对 14 个文件的 67 个 `historical` 字段。**51 一致、10 不一致、6 未核实。** 本表只记录在线核对结果，不回填 JSON、不修改游戏参数或 `_meta.status`。070a 的自动审计只检查 JSON 是否登记了来源（13 个已登记、54 个未登记或显式未核）；该数字不等于在线核实结果。刷新 070a 的结构清单时，把 `source-audit.mjs --write` 指向临时文件，不覆盖本表。

「一致」含名义弹种、四舍五入及注明配置的标称值；「不一致」表示文献与当前值在数值或字段定义上有冲突，不代表已修订；「未核实」表示没找到可确认当前值和对应口径的出处，即使有相关资料也不计为一致。来源页码为印刷页，另标 `PDF` 的除外；网页以节号、表名或馆藏号定位。质量、初速和射速按原来源的状态或定义对照，不把装填状态、周期射速、实用射速或游戏射速上限混用。

范围仅是原审计的 67 个 `historical` 字段：Sherman 外廓尺寸三项合计一行；`gameplay` 的装甲全表、装填、携弹配额与对标复测不计入，不据此判 E1-16 通过。差异和回填待办见 [PLAN_ISSUES.md](../PLAN_ISSUES.md) 的 PI-124G。

## 枪械

| 文件 | JSON 状态 | 字段 | JSON 值 | 核对结果 | 来源及定位 | 对照说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `weapons/ger_kar98k.json` | `reviewed` | `caliber` | `7.92×57 mm` | 一致 | [SS14] §II.3.c(2)；[SEAFORTH] 馆藏 1976.001.001a | 名义口径为 7.92 mm；`7.92×57 mm` 是膛室/弹规格简写。 |
| `weapons/ger_kar98k.json` | `reviewed` | `cartridge` | `7.92×57 mm Mauser (s.S.)` | 一致 | [WB] 第 54 页表 9；[SEAFORTH] 馆藏说明 | 美军医务史明确把 Patr. s.S. 列为 Kar. 98 K 所用弹种之一；不是唯一制式装药。 |
| `weapons/ger_kar98k.json` | `reviewed` | `magazine_capacity` | `5` | 一致 | [SS14] §II.3.c(2)；[BW] 第 235 页 Karabiner 98 | 固定弹仓 5 发。 |
| `weapons/ger_kar98k.json` | `reviewed` | `magazine_type` | `stripper_clip_internal_box` | 一致 | [SS14] §II.3 图 12 及 d(2)；[TME] §VII.II.5.a(2) | 桥夹与内置弹仓不是同一零件。 |
| `weapons/ger_kar98k.json` | `reviewed` | `muzzle_velocity_mps` | `760` | 未核实 | [SS14] §II.3.c(2)；[BICC] 第 47 页 K98k 规格；[WB] 德军小武器弹药段 | 未找到同一装药与测试口径的一手资料确认 760；SS14 未指明弹种、记约 853 m/s，BICC 记 755 m/s；WB 给该类弹药跨武器约 732–762 m/s，未把 760 指定给此枪。 |
| `weapons/ger_kar98k.json` | `reviewed` | `rate_of_fire_rpm_practical` | `15` | 未核实 | [BICC] 第 47 页 K98k 规格；[SS14] §II.3.c(2) 未列射速 | BICC 记约 15 发/分，但没有定义实用射速或测试条件，不能核定本字段。 |
| `weapons/ger_kar98k.json` | `reviewed` | `mass_kg` | `3.9` | 一致 | [BW] 第 235 页 Karabiner 98；[SS14] §II.3.c(2) | 德联邦国防军的礼仪用 K98k 枪体为 3.9 kg；战时美方情报仅给约 9 lb（约 4.08 kg），不推广至全部木托和配置。 |
| `weapons/ger_mp40.json` | `initial` | `caliber` | `9×19 mm` | 一致 | [MPVH] 第 47 页技术数据表 | 名义口径 9 mm；`9×19 mm` 为膛室规格。 |
| `weapons/ger_mp40.json` | `initial` | `cartridge` | `9×19 mm Parabellum` | 一致 | [MPVH] 第 47 页；[TME] §VII.II.4.a(3) | 9×19 mm Parabellum。 |
| `weapons/ger_mp40.json` | `initial` | `magazine_capacity` | `32` | 一致 | [MPSS] 第 17 页 §II.3.c(2)；[MPVH] 第 47 页 | 每匣 32 发。 |
| `weapons/ger_mp40.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [MPSS] 第 17 页 §II.3.c(2)；[MPPARA] Specifications | 可拆卸盒式弹匣。 |
| `weapons/ger_mp40.json` | `initial` | `muzzle_velocity_mps` | `380` | 一致 | [MPVH] 第 47 页；[MPMSO] PDF 第 3 页技术规格；[MPPARA] Specifications | 两处馆方资料给 380 m/s，属标称值；另一馆方目录给 400 m/s，未统一弹药或测试条件。 |
| `weapons/ger_mp40.json` | `initial` | `rate_of_fire_rpm_practical` | `500` | 不一致 | [TME] §VII.II.4.a(2)；[MPSS] 第 17 页 §II.3.c(2) | 500 是周期射速；1945 年情报给实用 180，1943 年情报在短点射口径给 80–90。不可把游戏上限当实用值。 |
| `weapons/ger_mp40.json` | `initial` | `mass_kg` | `4.0` | 一致 | [MPMSO] PDF 第 3 页技术规格；[TME] §VII.II.4.a(2) | 馆方空载标称 4.03 kg，按 0.1 kg 取整为 4.0 kg；美军情报另给不带弹匣约 4.08 kg。 |
| `weapons/ger_p38.json` | `initial` | `caliber` | `9×19 mm` | 一致 | [PHDV] 第 5 页 §1；[CIP9] Tab. IV 别名 | 德军手册名义口径为 9 mm；`9×19 mm` 为膛室规格。 |
| `weapons/ger_p38.json` | `initial` | `cartridge` | `9×19 mm Parabellum` | 一致 | [PHDV] 第 31 页 §28；[PSS] §II.2.c(2)；[CIP9] Tab. IV 别名 | 德军手册称 Pistolen-Patrone 08；美军情报与 C.I.P. 的别名表支持 Parabellum、9×19 的对应。 |
| `weapons/ger_p38.json` | `initial` | `magazine_capacity` | `8` | 一致 | [PHDV] 第 12 页 §9 | 每匣 8 发。 |
| `weapons/ger_p38.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [PHDV] 第 12 页 §9；[PSS] §II.2.c(1) | 德军手册列盒式，美军情报确认可拆卸。 |
| `weapons/ger_p38.json` | `initial` | `muzzle_velocity_mps` | `350` | 一致 | [PLHM] 文物说明首段；[PSOV] 第 4 页基本数据；[PSS] §II.2.c(2) | 与利沃夫历史博物馆的标称值一致；苏军手册记 330 m/s、美军情报约 317 m/s，未统一试验口径。 |
| `weapons/ger_p38.json` | `initial` | `rate_of_fire_rpm_practical` | `300` | 不一致 | [PWA] PDF 第 1 页 Zahlenangaben §3 | 1941 年德军表给实用 32–40 发/分；300 是游戏上限，不是实用射速。 |
| `weapons/ger_p38.json` | `initial` | `mass_kg` | `0.96` | 一致 | [PSS] §II.2.c(2)；[PWA] PDF 第 1 页 Zahlenangaben §1；[PSOV] 第 4 页 | 美军情报带空匣约 0.957 kg，四舍五入为 0.96 kg；德军表裸枪与苏军手册带空匣另有不同值。 |
| `weapons/us_m1911a1.json` | `initial` | `caliber` | `11.43×23 mm` | 一致 | [FM35] 第 2 页 §2.a；[CIP45] Tab. IV | 名义口径 .45 英寸；`11.43×23 mm` 是膛室规格简写。 |
| `weapons/us_m1911a1.json` | `initial` | `cartridge` | `.45 ACP` | 一致 | [TM2200] 第 196–197 页 §113；[CIP45] Tab. IV 别名 | 战时手册列 .45 M1911 普通弹；C.I.P. 给 .45 ACP 别名。 |
| `weapons/us_m1911a1.json` | `initial` | `magazine_capacity` | `7` | 一致 | [FM35] §1.c；[TM2200] 第 7 页 §4 | 每匣 7 发。 |
| `weapons/us_m1911a1.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [FM35] §1.c 及图 3 | 可拆卸盒式弹匣。 |
| `weapons/us_m1911a1.json` | `initial` | `muzzle_velocity_mps` | `253` | 一致 | [TM2200] 第 196–197 页 §113.d；[FM35] 第 3 页 §2.d(3) | 战时弹药表中 M1911 普通弹、5 英寸枪管的 830 ft/s 约为 253 m/s；1940 年手册表列零距离值为 802 ft/s（约 244 m/s）。 |
| `weapons/us_m1911a1.json` | `initial` | `rate_of_fire_rpm_practical` | `300` | 未核实 | [FM35] §1.d | 战时手册称取决于射手和换匣，未提供固定值。300 为游戏上限，未找到同口径史实来源。 |
| `weapons/us_m1911a1.json` | `initial` | `mass_kg` | `1.1` | 一致 | [FM35] 第 2 页 §2.b；[TM2200] 第 7 页 §4 | 战时手册带空匣约 2.437 lb（约 1.11 kg）。 |
| `weapons/us_m1_carbine.json` | `initial` | `caliber` | `7.62×33 mm` | 一致 | [FM7] 第 1 页 §2.a；[CIPCARB] Tab. I | 名义口径 .30 英寸；`7.62×33 mm` 是膛室规格简写，C.I.P. 壳长上限 32.77 mm。 |
| `weapons/us_m1_carbine.json` | `initial` | `cartridge` | `.30 Carbine` | 一致 | [TM2200] 第 178–179 页 §101；[CIPCARB] Tab. I | .30 Carbine，战时弹药表列 M1 卡宾枪用弹；不是 Garand 的 .30-06。 |
| `weapons/us_m1_carbine.json` | `initial` | `magazine_capacity` | `15` | 一致 | [FM7] 第 1 页 §1；[TM2200] 第 13 页 §7 | 战时 M1 的 15 发弹匣。 |
| `weapons/us_m1_carbine.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [FM7] 第 1 页 §1；[TM2200] 第 13 页 §7 | 可拆卸的盒式弹匣。 |
| `weapons/us_m1_carbine.json` | `initial` | `muzzle_velocity_mps` | `580` | 一致 | [FM7] 第 3 页 §3；[TM2200] 第 178–179 页 §101.d | 1942 年 FM 列 1,900 ft/s（约 579 m/s）；1943 年技术手册另列 M1 普通弹 1,975 ft/s（约 602 m/s）。 |
| `weapons/us_m1_carbine.json` | `initial` | `rate_of_fire_rpm_practical` | `40` | 未核实 | [FM7] 第 99 页 §74.b | 1942 年手册在指定速射课目写约 30 发瞄准射击/分，不能确认为通用的 40 发/分实用值。 |
| `weapons/us_m1_carbine.json` | `initial` | `mass_kg` | `2.4` | 一致 | [FM7] 第 1 页 §2.b | 不带背带、带空匣约 5.29 lb（约 2.40 kg）；带背带和满匣约 5.80 lb（约 2.63 kg）。 |
| `weapons/us_m1_garand.json` | `reviewed` | `caliber` | `7.62×63 mm` | 一致 | [TM1275] 第 5–6 页 §5.a；[CIP30] Tab. I 别名 | 手册列 .30 口径；`7.62×63 mm` 是 C.I.P. 所列弹规格别名，不是膛径。 |
| `weapons/us_m1_garand.json` | `reviewed` | `cartridge` | `.30-06 Springfield` | 一致 | [TM1275] 第 6 页 §5.a；[FM23-5-51] 第 122 页 §67；[CIP30] Tab. I 别名 | 手册列 M2 普通弹；`.30-06 Springfield` 是该弹规格名称，不等于限定只使用 M2。 |
| `weapons/us_m1_garand.json` | `reviewed` | `magazine_capacity` | `8` | 一致 | [FM23-5-51] 第 4 页 §3.a；[TM222] 第 1-2 页 §1-5.a | 8 发整夹。 |
| `weapons/us_m1_garand.json` | `reviewed` | `magazine_type` | `en_bloc_clip` | 一致 | [FM23-5-TR] §I.4 技术数据；[TM1275] 第 5 页 §5.a | 整夹式（en bloc），不是普通桥夹。 |
| `weapons/us_m1_garand.json` | `reviewed` | `muzzle_velocity_mps` | `853` | 一致 | [FM23-5-51] 第 122 页 §67；[FM23-5-TR] §I.4 | 1951 年手册的 M2 普通弹 2,800 ft/s，约 853 m/s，不套用于其他弹种。 |
| `weapons/us_m1_garand.json` | `reviewed` | `rate_of_fire_rpm_practical` | `24` | 一致 | [TM222] 第 1-2 页 §1-5.a；[FM23-5-TR] §I.4 | 24 是战后手册所列瞄准射击 16–24 发/分的上限，不是周期射速；GD §6.2 的 40 仍按 PI-012 处理。 |
| `weapons/us_m1_garand.json` | `reviewed` | `mass_kg` | `4.3` | 一致 | [TM1275] 第 5 页 §5.a；[TM222] 第 1-2 页 §1-5.a | 无附件约 9.5 lb（约 4.31 kg）。 |
| `weapons/us_m1_garand.json` | `reviewed` | `effective_range_m` | `460` | 一致 | [FM23-5-51] 第 4–5 页 §3.b；[FM23-5-TR] §I.4 | 1951 年版 500 yd（约 457 m）；转录版直接列 460 m，为标称取整值。 |
| `weapons/us_m3_grease.json` | `initial` | `caliber` | `11.43×23 mm` | 一致 | [TM2200] 第 55 页 §28；[CIP45] Tab. IV | 这里是 .45 口径 M3；`11.43×23 mm` 为膛室规格简写。 |
| `weapons/us_m3_grease.json` | `initial` | `cartridge` | `.45 ACP` | 一致 | [FM41L] 第 3 页 §3.a；[TM2200] 第 196–197 页 §113；[CIP45] Tab. IV 别名 | .45 M1911 普通弹，对应 .45 ACP。 |
| `weapons/us_m3_grease.json` | `initial` | `magazine_capacity` | `30` | 一致 | [TM2200] 第 55 页 §28；[FM41L] 第 4 页 §3.b | 每匣 30 发。 |
| `weapons/us_m3_grease.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [TM2200] 第 54–55 页图 26、§28 | 可拆卸盒式弹匣。 |
| `weapons/us_m3_grease.json` | `initial` | `muzzle_velocity_mps` | `280` | 未核实 | [TM2200] 第 55 页 §28、第 196–197 页 §113.d；[FM41L] 第 6 页 §4.c | 战时技术手册的 920 ft/s（约 280 m/s）是 10.5 英寸枪管参考值，而 M3 条目列 8 英寸；不能直接套用；战后 FM 的 M3/M3A1 合列约 274 m/s，未证实 280。 |
| `weapons/us_m3_grease.json` | `initial` | `rate_of_fire_rpm_practical` | `450` | 不一致 | [TM2200] 第 55 页 §28；[FM41L] 第 6 页 §4 | 450 是 350–450 发/分周期射速的上限，不是实用射速。 |
| `weapons/us_m3_grease.json` | `initial` | `mass_kg` | `3.7` | 一致 | [TM2200] 第 55 页 §28；[FM41L] 第 6 页 §4 | 不含弹匣、油壶与背带约 8.1 lb（约 3.67 kg）；含这些附件约 8.9 lb（约 4.04 kg）。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `caliber` | `11.43×23 mm` | 一致 | [TM2200] 第 57 页 §29；[CIP45] Tab. IV | 名义口径 .45 英寸；`11.43×23 mm` 为膛室规格简写。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `cartridge` | `.45 ACP` | 一致 | [TM2200] 第 57 页 §29、第 196–197 页 §113；[CIP45] Tab. IV 别名 | .45 M1911 普通弹，对应 .45 ACP。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `magazine_capacity` | `30` | 一致 | [TM2200] 第 56–57 页图 27、§29 | 30 发是战时手册列出的容量之一；另有 20 发。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `magazine_type` | `detachable_box` | 一致 | [TM2200] 第 56–57 页图 27、§29 | M1A1 条目列出 20/30 发盒式弹匣，不套用 M1928A1 的弹鼓规格。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `muzzle_velocity_mps` | `280` | 一致 | [TM2200] 第 57 页 §29、第 196–197 页 §113.d | 战时弹药表以 10.5 英寸枪管列 920 ft/s（约 280 m/s）；与 M1A1 的 10.52 英寸枪管近似，为同枪管长度的弹种参考值。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `rate_of_fire_rpm_practical` | `700` | 不一致 | [TM2200] 第 57 页 §29；[NCJ] Thompson M1A1 说明 | 700 是 700–800 发/分周期射速的下限，不能当作实用射速。 |
| `weapons/us_thompson_m1a1.json` | `initial` | `mass_kg` | `4.8` | 一致 | [TM2200] 第 57 页 §29 | 战时手册裸枪约 10 lb，另列 30 发弹匣 0.5 lb；合计约 4.76 kg，取整 4.8 kg（并非满匣）。 |

## 投掷物与反坦克装备（目录整件质量）

| 文件 | JSON 状态 | 字段 | JSON 值 | 核对结果 | 来源及定位 | 对照说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `explosives/ger_m24.json` | `initial` | `mass_kg` | `0.595` | 一致 | [ENORD] 第 321 页 Model 24，Weight | M24 整件 1 lb 5 oz（约 0.5953 kg）。 |
| `explosives/ger_nb39.json` | `initial` | `mass_kg` | `0.5` | 不一致 | [ENORD] 第 322 页 Smoke Hand Grenade 39，Total weight | 来源的发烟型整件总重 1 lb 14 oz（约 0.8505 kg），不是 0.5 kg；不可套用同目录的爆炸型 Model 39。 |
| `explosives/ger_panzerfaust_60.json` | `initial` | `mass_kg` | `6.25` | 未核实 | [ENORD] PDF 第 6 页，印刷第 218 页勘误 | 勘误区分了 60 型与两个 30 型，但未给 60 型整件质量；不能套用其他型号。 |
| `explosives/us_m18.json` | `initial` | `mass_kg` | `0.54` | 不一致 | [FM30] 第 15 页 Hand Grenades；[TM43] 第 2-27 页 Tabulated Data | 1944 年手册合列 M18/M16 的整件标称 17 oz（约 0.482 kg）；战后 M18 目录才给 19 oz（约 0.539 kg），与 JSON 取整相符。 |
| `explosives/us_mk2.json` | `initial` | `mass_kg` | `0.595` | 不一致 | [FM30] 第 14 页 Hand Grenades；[TM43] 第 2-19 页 Tabulated Data | 1944 年手册列同名型整件 20 oz（约 0.567 kg）；战后目录列 21 oz（约 0.595 kg），不代表相同战时配置。 |

## M4A1 Sherman (75)

| 文件 | JSON 状态 | 字段 | JSON 值 | 核对结果 | 来源及定位 | 对照说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `vehicles/us_m4a1_75.json` | `initial` | `mass_kg` | `30300` | 不一致 | [TM2800B] 第 109 页 General Data、Additional Data | 战时目录的 67,300 lb（约 30,527 kg）是目录注明履带配置的战斗全重；不据此推至其他配置。 |
| `vehicles/us_m4a1_75.json` | `initial` | `crew` | `5` | 一致 | [TM2800B] 第 109 页 General Data | 史实乘员为 5；游戏座位数 3 不属该字段。 |
| `vehicles/us_m4a1_75.json` | `initial` | `main_gun` | `75 mm M3` | 一致 | [TM2800A] 第 108 页 Armament | 此目录的 75 mm 型为 M3；不套用 76 mm 车型。 |
| `vehicles/us_m4a1_75.json` | `initial` | `max_speed_mps` | `10.5556` | 不一致 | [TM2800B] 第 109 页 Performance | 目录最高允许速度 24 mph（约 10.729 m/s、38.624 km/h），不是 JSON 的约 38.000 km/h；不是巡航速度。 |
| `vehicles/us_m4a1_75.json` | `initial` | `dimensions_m` | `5.84 / 2.62 / 2.74` | 不一致 | [TM2800A] 第 108 页尺寸图 | 长/宽/高：无防沙板为 5.842 / 2.591 / 2.845 m。JSON 长度吻合、宽度近有防沙板构型，但有防沙板时长度也变为 6.223 m；JSON 高度与图示不符。 |

## 在线来源

链接指向所用原始文献扫描、转录或发布机构的馆藏/资料页；没有复制手册正文。

[SS14]: https://www.lonesentry.com/manuals/german-infantry-weapons/kar98-german-rifle.html "U.S. Military Intelligence Service, Special Series No. 14 (1943), §II.3，转录"
[TME]: https://www.ibiblio.org/hyperwar/Germany/HB/HB-7.html "U.S. War Department TM-E 30-451 (1945), Ch. VII，转录"
[SEAFORTH]: https://www.seaforthhighlanders.ca/museum-exhibits/1976.001.001a "Seaforth Highlanders Museum, Mauser KAR 98K 馆藏"
[WB]: https://achh.army.mil/history/book-wwii-woundblstcs-chapter1/ "U.S. Army Medical Department, Wound Ballistics in World War II, Ch. I"
[BW]: https://www.bundeswehr.de/resource/blob/5031824/7b51f1e2b8e19099dacc615881c591a4/waffensysteme-und-grossgeraet-data.pdf "Bundeswehr, Waffensysteme und Großgerät，服役礼仪用配置"
[BICC]: https://bicc.de/Publikationen/SALW_Guide_2021_Edition-WEB.pdf "BICC, Small Arms and Light Weapons Identification Guide (2021)，识别指南"
[MPVH]: https://www.mosr.sk/data/att/107241.pdf#page=47 "斯洛伐克军事史研究所 MP 40 技术表"
[MPMSO]: https://lechevron.fr/images/articles/publications/lettre/lettre_MSO_008.pdf#page=3 "Musée du Sous-Officier，MP 40 技术规格"
[MPSS]: https://www.lonesentry.com/manuals/german-infantry-weapons/mp40-machine-pistol.html "U.S. Military Intelligence Service, Special Series No. 14 (1943)，转录"
[MPPARA]: https://paradata.org.uk/content/4663887-mp40-sub-machine-gun "Airborne Assault Museum, MP40 specifications"
[PHDV]: https://bestanden.pistole38.nl/manuals/German-P38-manual-ww2-HdV254.pdf "H.Dv. 254 (1940)，德国手册扫描"
[PSS]: https://www.lonesentry.com/manuals/german-infantry-weapons/walther-pistol.html "U.S. Military Intelligence Service, Special Series No. 14 (1943)，转录"
[PWA]: https://bestanden.pistole38.nl/manuals/P38-manual-Waffentafeln-Potsdam-1941.pdf "Potsdam Waffentafel Wa 42/43 (1941)，德国资料扫描"
[PSOV]: https://bestanden.pistole38.nl/manuals/Soviet-P38-manual-1942.pdf "苏联红军炮兵学院 P38 手册 (1942)，扫描"
[PLHM]: https://artsandculture.google.com/asset/pistol-walther-p38-carl-walther-gmbh/0AGT2TTSnOMFow?hl=en "Lviv Historical Museum，P38 馆藏"
[CIP9]: https://bobp.cip-bobp.org/uploads/tdcc/tab-iv/tabivcal-en-page28.pdf "C.I.P. TDCC Tab. IV, 9 mm Luger"
[CIP30]: https://bobp.cip-bobp.org/uploads/tdcc/tab-i/tabical-en-page111.pdf "C.I.P. TDCC Tab. I, 30-06 Spring."
[TM1275]: https://www.govinfo.gov/content/pkg/GOVPUB-W-ef05ae6d667106da89ecdffc521f4929/pdf/GOVPUB-W-ef05ae6d667106da89ecdffc521f4929.pdf#page=11 "U.S. War Department TM 9-1275 (1947)，GPO 扫描"
[FM23-5-51]: https://archive.org/download/fm-23.5-u.s.-rifle-caliber-.30-m1-1951/FM_23-5_U.S._Rifle_Caliber_.30%2C_M1_1951.pdf "U.S. Army and Air Force FM 23-5 (1951)，扫描"
[FM23-5-TR]: https://biggerhammer.net/manuals/garand/intro.htm "FM 23-5，战后转录，页面未标版次"
[TM222]: https://www.airforcebase.net/uploads/marksmanship/TM_9-1005-222-12_Op_Org_Mx_Rifle_Caliber_.30_M1_M1C_M1D_19690317.pdf "U.S. Army TM 9-1005-222-12 (1969)，扫描"
[FM35]: https://www.ibiblio.org/hyperwar/USA/ref/FM/PDFs/FM23-35.pdf "U.S. War Department FM 23-35 (1940)，扫描"
[FM7]: https://patcosta.com/wp-content/uploads/2024/07/FM-23-7-M1-Carbine-May-1942.pdf "U.S. War Department FM 23-7 (1942)，扫描"
[TM2200]: https://www.ibiblio.org/hyperwar/USA/ref/TM/pdfs/TM9-2200.pdf "U.S. War Department TM 9-2200 (1943)，扫描"
[CIP45]: https://bobp.cip-bobp.org/uploads/tdcc/tab-iv/45-auto-en-1.pdf "C.I.P. TDCC Tab. IV, 45 Auto"
[CIPCARB]: https://bobp.cip-bobp.org/uploads/tdcc/tab-i/tabical-en-page106.pdf "C.I.P. TDCC Tab. I, 30 Carbine"
[ENORD]: https://stephentaylorhistorian.com/wp-content/uploads/2020/02/catalogofenemyordnancemateriel.pdf "U.S. Office of the Chief of Ordnance, Catalog of Enemy Ordnance Materiel，扫描"
[TM43]: https://www.bulletpicker.com/pdf/TM-43-0001-29.pdf "U.S. Army TM 43-0001-29, Army Ammunition Data Sheets for Grenades，战后版扫描"
[TM2800A]: https://archive.org/download/TM9-2800/TM9-2800.pdf#page=108 "U.S. War Department TM 9-2800 (1943)，M4A1 尺寸图与型号"
[TM2800B]: https://archive.org/download/TM9-2800/TM9-2800.pdf#page=109 "U.S. War Department TM 9-2800 (1943)，M4A1 规格表"
[FM30]: https://www.ibiblio.org/hyperwar/USA/ref/FM/PDFs/FM23-30.pdf#page=11 "U.S. War Department FM 23-30 (1944)，整件质量表，扫描"
[FM41L]: https://www.biggerhammer.net/manuals/fm23_41/CH1.PDF "U.S. Army FM 23-41，战后 M3/M3A1 手册，第 1 章扫描"
[NCJ]: https://ojp.gov/pdffiles1/Digitization/9639NCJRS.pdf "Submachine Guns in Police Operations，NCJRS 数字馆藏，战后周期射速对照"
