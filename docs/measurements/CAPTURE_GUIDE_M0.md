# 批 M0 对标录像：负责人一页清单

> 给人类负责人（NEEDS_HUMAN H-011 / H-025 / H-026）。本页只讲"录什么、什么设置、放哪里、录完跑哪条命令"。
> 方法依据：`docs/REFERENCE_MEASUREMENTS.md` §1（合规）、§2（采集）、§4（待测项）。**测量值栏全部是"待测"，工具链已就绪，录像是唯一缺口。**
> 本页中参考作品一律称"参考作品"；不得把它的名称、截图、录像或音频放进仓库或任何对外材料。

## 0. 开录前必须先确认（任一项没确认就不要录）

1. **H-011**：合法获取参考作品，通读 EULA，确认**允许录制与内部分析**。不允许 → 停止，改用公开资料（§5）。
2. **H-025**：录制机能**稳定** ≥ 120 fps（目标 240 fps）、1080p 原生渲染。游戏渲染帧率必须稳定 ≥ 采集帧率（否则录像里有重复帧）；不够就降画质，**不要降采集帧率**。
3. **私有存储**（§1.3）：决定录像放哪、谁能访问、保留多久。录像**不进仓库、不上传任何公共或不受控的服务**。
   仓库已忽略 `docs/measurements/raw/` 与 `assets/incoming/`，但最好把录像放在仓库之外，并设环境变量：
   `export REF_CAPTURE_DIR=/路径/到/私有录像目录`

## 1. 录制设置（全程不变，每个会话开头自检一次）

| 项 | 设置 |
|---|---|
| 帧率 / 分辨率 | ≥ 120 fps（目标 240）/ 1920×1080，关动态分辨率与超分；**恒定帧率**（OBS 里选 CFR，不是 VFR） |
| 码率 / 格式 | ≥ 80 Mbps 或近无损（CQP 12–16）；MKV 容器；不要二次有损转码 |
| 音频 | 游戏音单独一轨，48 kHz，PCM 或 ≥ 256 kbps AAC；关麦克风、语音聊天、背景音乐 |
| 画面 | **关动态模糊**；关景深/镜头光晕/屏幕抖动（压制测量除外，本批不含）；HUD 保持默认 |
| FOV | 固定，**记下游戏内 FOV 设置值**，整个批次不改 |
| 鼠标 | 关加速；记下 DPI × 游戏内灵敏度 |
| 垂直同步 | 关；帧率上限 = 采集帧率的整数倍 |
| 必须记下 | 参考作品**版本/构建号**（每次录前、录后各看一次，关掉自动更新）、日期、录制软件版本 |
| 每段前后 | 各留 **5 秒静止空白**（校时与噪声） |

## 2. 会话安排（至少 3 个会话，每项每次重复 ≥ 5 次；后坐/扩散/致死发数 ≥ 10 次）

重复 = **每次重新开始**（重新出生/重新装填/重新进入姿态），不要录一大段再切；除非该项就是长连射。

**会话 A　步枪射击（S-RANGE，练习场/空服/私人服）**　两把首发步枪各做一遍：美军 M1 Garand（半自动）、德军 Kar98k（栓动）。
- 近距离（≤ 5 m）对空地/墙长连射，**打完一整个弹匣并换弹后继续**，累计 ≥ 30 发 → W-01、W-02（Garand 连点最快）、W-13（Kar98k 拉栓）；这些长序列一定要**音轨干净**。
- 空仓换弹、战术换弹各 5 次 → W-03、W-04；Kar98k 逐发/夹装装填与中途打断 → W-05、W-06。
- 开镜进入/退出各 5 次（侧面有固定远景地物，便于看 FOV 稳定）→ W-07、W-08、W-09（腰射与开镜各做一次指南针/远景转身标定）。
- 切枪（若首发有第二把武器）→ W-11、W-12。
- 固定远景地物，腰射/开镜连发各 ≥ 10 次 → W-15–W-18（后坐）；固定靶 ≥ 30 发腰射、开镜（站/蹲）→ W-19–W-21（扩散）。
- 100/200/300 m（及 400/500 m）靶标：落点偏移 → W-22–W-25；曳光/命中反馈飞行时间 → W-22；是否可见弹丸 → W-26。
- ADS 静止站立 ≥ 10 s（不屏息）、各姿态、屏息到耗尽再恢复 → W-27–W-29。
- 读 HUD 弹匣与总弹量 → W-30。

**会话 B　基础移动（S-FIELD 平坦开阔地，地图网格可见；S-OBST 低/中/高矮墙）**
- 沿 ≥ 100 m 直线匀速：走、跑、冲刺、蹲行、匍匐、后退、侧移，各 5 次，**让身体穿过两个已知网格点**，起止点距离 ≥ 100 m → M-01–M-06。
- 站→蹲、蹲→站、站/蹲→趴、趴→站/蹲各 5 次（侧面或第一人称看地平线/眼高变化）→ M-12–M-14。
- 不同高度矮墙逐个尝试翻越 → M-17；原地起跳（侧面机位 + 已知高度参照）→ M-19。
- 从静止直线起步、稳定速度下松键、稳定前进时瞬间反向 → M-24、M-25、M-26（用第三人称/观战视角或固定地物做帧级位移更好）。

**会话 C　生存基础（S-DUMMY，需要第二账号做靶；H-027 协助者）**
- 每把步枪对站立靶：近 50 m / 中 150 m / 远 300 m 各打到倒下，每距离 ≥ 10 次，记发数 → H-01（躯干）；瞄头 → H-02；瞄四肢 → H-03；W-37 与它们共用录像。
- 靶的血条/受击数字若能读，直接读；否则按"打到倒下需要几发"记。

## 3. 文件命名与放哪里

```
{项目ID}_{版本}_{日期}_{场景}_{序号}.mkv        例（格式示例，非真实数据）：W-01.us_m1_garand_v0.0.0_2026-01-01_S-RANGE_r03.mkv
```
放进私有目录（`$REF_CAPTURE_DIR`，或 `docs/measurements/raw/`——已被 git 忽略）。武器类项目 ID 带武器后缀，H-01 带距离后缀（`H-01.us_m1_garand@50m`）。

## 4. 录像到位后怎么跑（分析工具：`tools/ref-analysis/`）

```bash
pip install -r tools/ref-analysis/requirements.txt           # numpy / scipy / opencv-headless / pytest；另需 ffmpeg >= 6
export PYTHONPATH=tools/ref-analysis REF_CAPTURE_DIR=/路径/到/私有录像目录
(cd tools/ref-analysis && python3 -m pytest -q)               # 先确认工具自检全绿（69 项）

python3 -m ref_analysis probe  W-01..._r01.mkv                # 恒定帧率检查：退出码 2 = VFR，必须丢弃重录
python3 -m ref_analysis audio-rpm  W-01..._r01.mkv            # 射速：音频峰值法（W-01、W-02、W-13）
python3 -m ref_analysis frame-rpm  W-01..._r01.mkv --roi W:H:X:Y   # 同一段用枪口火光首帧交叉验证，两法应 ±2 %
python3 -m ref_analysis speed --frame-a 812 --frame-b 3552 --distance 100 --fps 120   # M-CAL-D：两个网格点之间的速度
python3 -m ref_analysis track  M-24..._r01.mkv --roi W:H:X:Y --px-per-m 100 --csv out.csv   # 模板/亮点跟踪（加减速）
python3 -m ref_analysis fov --x "px1,px2,..." --theta="deg1,deg2,..." --width 1920   # FOV 标定（负角度必须用 --theta=）
python3 -m ref_analysis stats "v1,v2,v3,v4,v5" --frame-unc 0.0083      # 中位数/极差/不确定度/置信度
```

每一项把各次重复的原始值（来自上面的命令）写进一个 JSON，再一条命令生成 `docs/measurements/{ID}.md`（模板 `_TEMPLATE.md`）
和 `summary.csv` 的一行（列定义：REFERENCE_MEASUREMENTS §8.2；按 (id, ref_version) 幂等更新；统计量、不确定度 = max(极差/2, 1 帧)、置信度 A–D 自动算）：

```bash
cat > spec.json <<'JSON'
{"row_id": "W-03.us_m1_garand", "ref_version": "<构建号>", "date": "2026-01-01", "fps": 240,
 "capture": "240 fps / 1920x1080 / 100 Mbps", "fov": "<游戏内 FOV>", "measurer": "<代号>", "analyst": "<代号>",
 "reps": [{"file": "<私有文件名>", "start_frame": 812, "end_frame": 1484, "value": 2.8},
          {"file": "<...>", "value": 2.82, "excluded": true, "reason": "按键失误，换弹被打断"}],
 "cross_validation": "无第二种方法", "public_source_agrees": "na"}
JSON
python3 -m ref_analysis record spec.json        # （格式示例，数值非真实数据；也可传一个对象数组一次写多项）
```

无法黑盒测得的项：`{"row_id": "M-17", ..., "infeasible": true, "notes": ["原因"]}` → status=infeasible、置信度 D。
序列型结果（后坐模式等）另存 `docs/measurements/series/{ID}.csv`（列 `index_or_t, value, unit, rep`）。

对标完成后与我方同法复测、核对完整性（任何时候都可以跑）：

```bash
OWN_REMEASURE_OUT=docs/measurements/own_m0.json cargo test -p frontline-sim --test own_remeasure   # 我方各项实测（已入库一份）
python3 -m ref_analysis own-fill docs/measurements/own_m0.json   # 把我方值、误差、容限、pass 填进 summary.csv（§7.3 容限）
python3 -m ref_analysis validate                                 # summary.csv：25 列、枚举值
python3 -m ref_analysis status -v                                # 批 M0 是否每项都有 measured/infeasible 行与记录文件（缺项时退出码 1）
```

## 5. 批 M0 全部行（共 78 行）

| 行 ID | 项 | 场景 | 方法 | 重复 | 采集要点 |
|---|---|---|---|---|---|
| W-01.us_m1_garand | 实际射速（长连射平均） | S-RANGE | M-AUD;M-FRM | >= 5 | 近距离(<=5 m)连续点射/拉栓打完弹匣的长序列，单独音轨；半自动/栓动取最快可持续节奏 |
| W-01.ger_kar98k | 实际射速（长连射平均） | S-RANGE | M-AUD;M-FRM | >= 5 | 近距离(<=5 m)连续点射/拉栓打完弹匣的长序列，单独音轨；半自动/栓动取最快可持续节奏 |
| W-02.us_m1_garand | 半自动最快点射间隔 | S-RANGE | M-AUD | >= 5 | 尽可能快地连点开火键，>= 20 发（弹匣打空换弹后再继续） |
| W-03.us_m1_garand | 空仓换弹时间 | S-RANGE | M-FRM | >= 5 | 打空后按换弹键 -> 可再次开火的首帧 |
| W-03.ger_kar98k | 空仓换弹时间 | S-RANGE | M-FRM | >= 5 | 打空后按换弹键 -> 可再次开火的首帧 |
| W-04.us_m1_garand | 战术换弹时间 | S-RANGE | M-FRM | >= 5 | 弹匣未空时换弹 -> 可再次开火 |
| W-04.ger_kar98k | 战术换弹时间 | S-RANGE | M-FRM | >= 5 | 弹匣未空时换弹 -> 可再次开火 |
| W-05.ger_kar98k | 逐发装填每发时间与装满总时间 | S-RANGE | M-FRM | >= 5 | 栓动/逐发装填：记录每发装入的间隔与总时长 |
| W-06.ger_kar98k | 打断换弹的最早可开火时刻 | S-RANGE | M-FRM | >= 5 | 逐发装填中途按开火键 |
| W-07.us_m1_garand | 开镜进入时间 | S-RANGE | M-FRM;M-ANG | >= 5 | 按下开镜键 -> FOV/视图稳定的首帧 |
| W-07.ger_kar98k | 开镜进入时间 | S-RANGE | M-FRM;M-ANG | >= 5 | 按下开镜键 -> FOV/视图稳定的首帧 |
| W-08.us_m1_garand | 开镜退出时间 | S-RANGE | M-FRM;M-ANG | >= 5 | 松开开镜键 -> FOV/视图恢复稳定 |
| W-08.ger_kar98k | 开镜退出时间 | S-RANGE | M-FRM;M-ANG | >= 5 | 松开开镜键 -> FOV/视图恢复稳定 |
| W-09.us_m1_garand | ADS 放大倍率/FOV | S-RANGE | M-ANG | >= 5 | 腰射与开镜各做一次指南针/远景标定（3.5） |
| W-09.ger_kar98k | ADS 放大倍率/FOV | S-RANGE | M-ANG | >= 5 | 腰射与开镜各做一次指南针/远景标定（3.5） |
| W-11.us_m1_garand | 切枪时间：拔出 | S-RANGE | M-FRM | >= 5 | 按下切枪键 -> 可开火首帧 |
| W-11.ger_kar98k | 切枪时间：拔出 | S-RANGE | M-FRM | >= 5 | 按下切枪键 -> 可开火首帧 |
| W-12.us_m1_garand | 切枪时间：收起 | S-RANGE | M-FRM | >= 5 | 按下切枪键 -> 可拔下一把 |
| W-12.ger_kar98k | 切枪时间：收起 | S-RANGE | M-FRM | >= 5 | 按下切枪键 -> 可拔下一把 |
| W-13.ger_kar98k | 栓动单次循环时间 | S-RANGE | M-FRM | >= 5 | 开火 -> 上膛完成 -> 可再开火 |
| W-15.us_m1_garand | 后坐模式：竖直（前10发） | S-RANGE | M-TPL;M-ANG | >= 10 | 固定远景地物，腰射连发，n>=10 次；先做 FOV 标定；记录每发视角竖直位移（度） |
| W-15.ger_kar98k | 后坐模式：竖直（前10发） | S-RANGE | M-TPL;M-ANG | >= 10 | 固定远景地物，腰射连发，n>=10 次；先做 FOV 标定；记录每发视角竖直位移（度） |
| W-16.us_m1_garand | 后坐模式：水平（前10发） | S-RANGE | M-TPL;M-ANG | >= 10 | 同 W-15；统计量 = 每发水平位移绝对值的中位数（度），带符号序列存 series |
| W-16.ger_kar98k | 后坐模式：水平（前10发） | S-RANGE | M-TPL;M-ANG | >= 10 | 同 W-15；统计量 = 每发水平位移绝对值的中位数（度），带符号序列存 series |
| W-17.us_m1_garand | 后坐恢复：停火后回正时间与方式 | S-RANGE | M-TPL;M-FRM | >= 5 | 单发/连发后视角回正曲线 |
| W-17.ger_kar98k | 后坐恢复：停火后回正时间与方式 | S-RANGE | M-TPL;M-FRM | >= 5 | 单发/连发后视角回正曲线 |
| W-18.us_m1_garand | ADS 与腰射后坐倍率 | S-RANGE | M-TPL | >= 10 | 同一武器腰射 vs 开镜各 n>=10 |
| W-18.ger_kar98k | ADS 与腰射后坐倍率 | S-RANGE | M-TPL | >= 10 | 同一武器腰射 vs 开镜各 n>=10 |
| W-19.us_m1_garand | 腰射扩散 | S-RANGE | M-STAT | >= 30 | 固定靶，>= 30 发弹着点，转角度；每发间隔足够使后坐/扩散恢复。统计量 = 弹着点相对瞄准点的角偏差中位数（P95 记在备注） |
| W-19.ger_kar98k | 腰射扩散 | S-RANGE | M-STAT | >= 30 | 固定靶，>= 30 发弹着点，转角度；每发间隔足够使后坐/扩散恢复。统计量 = 弹着点相对瞄准点的角偏差中位数（P95 记在备注） |
| W-20.us_m1_garand | 开镜扩散（站/蹲/趴） | S-RANGE | M-STAT | >= 30 | 同 W-19，开镜 |
| W-20.ger_kar98k | 开镜扩散（站/蹲/趴） | S-RANGE | M-STAT | >= 30 | 同 W-19，开镜 |
| W-21.us_m1_garand | 扩散随移动/姿态的增长倍率（跑动/静止，中位角偏差之比） | S-RANGE | M-STAT;M-FRM | >= 30 | 静止/走/跑 x 站/蹲 各一组；比值 = 跑动中位角偏差 / 静止中位角偏差，走/蹲比值与恢复时间记在备注 |
| W-21.ger_kar98k | 扩散随移动/姿态的增长倍率（跑动/静止，中位角偏差之比） | S-RANGE | M-STAT;M-FRM | >= 30 | 静止/走/跑 x 站/蹲 各一组；比值 = 跑动中位角偏差 / 静止中位角偏差，走/蹲比值与恢复时间记在备注 |
| W-22.us_m1_garand | 弹头速度（曳光飞行时间法，>= 200 m） | S-RANGE | M-CAL;M-FRM | >= 5 | 枪口火光首帧 -> 命中靶标首帧，>= 200 m；记录 v = d / t（飞行平均速度，非初速），与我方同定义对比 |
| W-22.ger_kar98k | 弹头速度（曳光飞行时间法，>= 200 m） | S-RANGE | M-CAL;M-FRM | >= 5 | 枪口火光首帧 -> 命中靶标首帧，>= 200 m；记录 v = d / t（飞行平均速度，非初速），与我方同定义对比 |
| W-23.us_m1_garand | 下坠：100 m 落点偏移 | S-RANGE | M-CAL-F | >= 5 | 固定瞄准基准，带垂直刻度的靶 |
| W-23.ger_kar98k | 下坠：100 m 落点偏移 | S-RANGE | M-CAL-F | >= 5 | 固定瞄准基准，带垂直刻度的靶 |
| W-24.us_m1_garand | 下坠：200 m 落点偏移 | S-RANGE | M-CAL-F | >= 5 | 同 W-23 |
| W-24.ger_kar98k | 下坠：200 m 落点偏移 | S-RANGE | M-CAL-F | >= 5 | 同 W-23 |
| W-25.us_m1_garand | 下坠：300 m（及 400/500 m）落点偏移 | S-RANGE | M-CAL-F | >= 5 | 同 W-23 |
| W-25.ger_kar98k | 下坠：300 m（及 400/500 m）落点偏移 | S-RANGE | M-CAL-F | >= 5 | 同 W-23 |
| W-26.us_m1_garand | 弹头飞行形式（hitscan/弹丸） | S-RANGE | M-FRM | >= 5 | 观测记录：是否可见弹丸、命中反馈时间 |
| W-26.ger_kar98k | 弹头飞行形式（hitscan/弹丸） | S-RANGE | M-FRM | >= 5 | 观测记录：是否可见弹丸、命中反馈时间 |
| W-27.us_m1_garand | 准星晃动幅度与频率（ADS 站立，无屏息） | S-RANGE | M-TPL;M-ANG | >= 5 | ADS 静止 >= 10 s |
| W-27.ger_kar98k | 准星晃动幅度与频率（ADS 站立，无屏息） | S-RANGE | M-TPL;M-ANG | >= 5 | ADS 静止 >= 10 s |
| W-28.us_m1_garand | 晃动随姿态变化 | S-RANGE | M-TPL | >= 5 | 站/蹲/趴各一 |
| W-28.ger_kar98k | 晃动随姿态变化 | S-RANGE | M-TPL | >= 5 | 站/蹲/趴各一 |
| W-29.us_m1_garand | 屏息：抑制比、持续、恢复 | S-RANGE | M-TPL;M-FRM | >= 5 | 按住屏息至耗尽，再观察恢复 |
| W-29.ger_kar98k | 屏息：抑制比、持续、恢复 | S-RANGE | M-TPL;M-FRM | >= 5 | 按住屏息至耗尽，再观察恢复 |
| W-30.us_m1_garand | 弹匣容量与携弹量 | S-RANGE | M-UI;M-DOC | >= 5 | 读 HUD 弹药数 |
| W-30.ger_kar98k | 弹匣容量与携弹量 | S-RANGE | M-UI;M-DOC | >= 5 | 读 HUD 弹药数 |
| W-37.us_m1_garand | 伤害：各距离所需发数与部位倍率 | S-DUMMY | M-STAT | >= 10 | 与 H-01..H-03 同批录制（共用录像） |
| W-37.ger_kar98k | 伤害：各距离所需发数与部位倍率 | S-DUMMY | M-STAT | >= 10 | 与 H-01..H-03 同批录制（共用录像） |
| M-01 | 行走速度 | S-FIELD | M-CAL | >= 5 | 沿 >= 100 m 直线匀速走过两个网格点 |
| M-02 | 奔跑速度 | S-FIELD | M-CAL | >= 5 | 同 M-01 |
| M-03 | 冲刺速度 | S-FIELD | M-CAL | >= 5 | 同 M-01；体力允许的距离内 |
| M-04 | 蹲行速度 | S-FIELD | M-CAL | >= 5 | 同 M-01 |
| M-05 | 匍匐速度 | S-FIELD | M-CAL | >= 5 | 同 M-01 |
| M-06 | 后退/侧移速度倍率 | S-FIELD | M-CAL | >= 5 | 后退与侧移各一 |
| M-12 | 姿态切换：站->蹲 | S-FIELD | M-FRM | >= 5 | 按键 -> 视角/眼高稳定 |
| M-13 | 姿态切换：蹲->站 | S-FIELD | M-FRM | >= 5 | 同 M-12 |
| M-14 | 姿态切换：站/蹲->趴；趴->站/蹲 | S-FIELD | M-FRM | >= 5 | 同 M-12 |
| M-17 | 翻越矮墙时间与最高可翻越高度 | S-OBST | M-FRM;M-CAL | >= 5 | 不同高度矮墙逐个尝试 |
| M-19 | 跳跃高度与滞空时间 | S-OBST | M-FRM;M-CAL | >= 5 | 原地起跳，侧面机位+已知高度参照 |
| M-24 | 加速时间（静止->95 % 稳定速度） | S-FIELD | M-TPL;M-CAL | >= 5 | 从静止直线起步，帧级位移 |
| M-25 | 减速时间与滑行距离 | S-FIELD | M-TPL;M-CAL | >= 5 | 稳定速度下松键 |
| M-26 | 转向惯性：180 度反向时间 | S-FIELD | M-TPL | >= 5 | 稳定前进时瞬间反向 |
| H-01.us_m1_garand@50m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-01.us_m1_garand@150m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-01.us_m1_garand@300m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-01.ger_kar98k@50m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-01.ger_kar98k@150m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-01.ger_kar98k@300m | 致死发数（躯干，近/中/远） | S-DUMMY | M-STAT | >= 10 | 近 50 m / 中 150 m / 远 300 m 三个距离各一行（行 ID 带 @50m/@150m/@300m），每距离 n>=10 |
| H-02.us_m1_garand | 头部致死发数 | S-DUMMY | M-STAT | >= 10 | 同 H-01，瞄准头部 |
| H-02.ger_kar98k | 头部致死发数 | S-DUMMY | M-STAT | >= 10 | 同 H-01，瞄准头部 |
| H-03.us_m1_garand | 四肢致死发数/伤害倍率 | S-DUMMY | M-STAT | >= 10 | 同 H-01，瞄准四肢 |
| H-03.ger_kar98k | 四肢致死发数/伤害倍率 | S-DUMMY | M-STAT | >= 10 | 同 H-01，瞄准四肢 |

## 6. 常见坑

- 重复帧（游戏帧率 < 采集帧率）会让逐帧测量失真：先看 `probe` 的帧数是否等于 帧率 × 时长。
- 服务器按 tick 量化时序：射速等一律用**长连射的平均间隔**，不要看单个间隔。
- 距离读数要用地图网格交叉验证（§3.4）；**不得读取游戏文件**得到尺寸。
- 抽帧文件、音频 wav 只放私有目录，**不入库**；记录里只写数字和文字。
