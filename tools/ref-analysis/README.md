# tools/ref-analysis —— 对标录像分析工具链（M0-032）

黑盒录像分析（REFERENCE_MEASUREMENTS §3）+ 记录/汇总（§8）+ 我方同法复测对接（§7.4）。
**只读取私有录像、只输出数字/文字；不联网，不上传，不保存图像。** 需要 `ffmpeg`/`ffprobe` ≥ 6 在 PATH 上。

```bash
pip install -r tools/ref-analysis/requirements.txt      # numpy, scipy, opencv-python-headless, pytest
(cd tools/ref-analysis && python3 -m pytest -q)         # 69 项，约 30 s，全部用自己合成的视频/音频
export PYTHONPATH=tools/ref-analysis
python3 -m ref_analysis --help
```

| 模块 | 方法码 | 作用 |
|---|---|---|
| `common.py`, `extract_frames.py` | M-FRM 基础 | ffprobe（恒定帧率检查、±1 帧不确定度）、内存抽帧（`-fps_mode passthrough`，不丢帧不重复）、PNG 抽取（私有目录）、音频抽取 |
| `audio_events.py` | M-AUD | 1 ms RMS 包络 → 阈值段 → 回声合并 → 起音时刻 → `RPM = 60 (N-1) / (tN - t1)`；周期未知时用梳状评分估计 |
| `frame_events.py` | M-FRM | 感兴趣区域亮度序列 → 火光首帧 → 间隔 / 射速 / 事件差（±1 帧） |
| `calibrate.py` | M-CAL / M-TPL / M-ANG | 两点（网格）测速及误差传播；模板/亮点跟踪（亚像素）；Theil-Sen 速度；速度序列、95 % 加速时间、停止时间；FOV 拟合 `x = f tan(θ-θ0)+W/2`；像素→度；下坠 `tan θ·d`；过渡时长（开镜/姿态） |
| `suppression.py` | M-IMG / M-FLOW | Laplacian 方差曲线与关键点（起始延迟/峰值/恢复）；Farneback 光流镜头位移、RMS/峰峰值/主频；暗角、去饱和曲线（本批 M0 不含压制，M1 用） |
| `stats.py` | M-STAT | 中位数/极差/不确定度 `max(极差/2, 1 帧)`、置信度 A–D（§4.0）、速度误差传播、§7.3 容限与比较 |
| `items.py` | — | 批 M0 全部 78 行（§9.1：武器按首发步枪展开，H-01 带 `@50m/@150m/@300m`）；元数据只有"测什么/怎么测"，**没有任何参考数值** |
| `record.py` | — | `summary.csv`（25 列，按 §8.2 逐列校验，按 (id, ref_version) 幂等 upsert）、`{ID}.md` 记录、JSON spec → 记录 + 汇总行 |
| `own_compare.py` | §7.4 | 读 `own_remeasure` 导出的 JSON，填 `own_median / own_error_pct / tolerance_pct / pass`，渲染我方数值表 |
| `cli.py` | — | `probe audio-rpm frame-rpm speed track fov suppression stats record own-fill own-table status validate checklist` |

环境变量：`REF_CAPTURE_DIR`（相对路径的录像基准目录）、`REF_MEASUREMENTS_DIR`（默认 `docs/measurements`，测试里指向临时目录）。
负角度参数请写成 `--theta=-15,-10,...`（否则 argparse 把它当选项）。

## 测回精度（合成数据，知道真值）

| 工具 | 合成输入 | 实测 |
|---|---|---|
| `audio_events` | 600 rpm、31 发、48 kHz，含回声与噪声（SNR 40 / 20 / 10 dB），onset 与 peak 两种模式 | 600 rpm 误差 0.006 %（10 dB SNR 最差 0.005 %）；40–1200 rpm 全部 ≤ 1 %；经 mkv(pcm) 往返 0.006 %；30 Hz tick 量化间隔的平均误差 < 0.5 % |
| `frame_events` | 120 fps 下 600 rpm 火光、144 fps 下非整数帧间隔 | 600.000 rpm（0 误差）；144 fps 量化间隔 0 误差，且在 ±1 帧给出的不确定度之内；H.264 有损编码 20 个火光全部检出 |
| `calibrate`（速度） | 4.4 m/s 匀速色块，100 px/m，120 fps；无损 / H.264 | 4.4000 m/s（误差 < 0.001 %），亮点跟踪与模板跟踪同；两点法误差 ≤ 3 % 且真值落在 ±1 帧带内 |
| `calibrate`（加减速） | 线性加速 0.25 s / 减速 0.20 s | 95 % 加速时间 0.2333 s（真值 0.2375，差 4 ms < 2 帧）；停止时间 0.1917 s（真值 0.190） |
| `calibrate`（FOV） | 78° 水平 FOV，1280 px，视频中转动 50° | 78.016°（差 0.016°）；噪声 0.7 px 的数值拟合 60/90/110° 均 ≤ 1 % |
| `calibrate`（过渡） | 0.25 s 线性过渡，120 fps | 0.2417 s（差 8.3 ms = 1 帧） |
| `suppression` | 已知模糊曲线 / 4 px@6 Hz 抖动 | 无损链路与直接计算逐位一致（模糊曲线：起始延迟 16.7 ms、峰值 0.0017、恢复 583 ms 与真值一致）；有损（CRF 14）关键帧 ±1 帧；抖动峰峰值 8.035 px（真值 8.0）、主频 6.01 Hz（真值 6.0） |

（具体数字以 `pytest -s` 的打印为准，测试里有硬断言。）

## 我方复测（Rust，`crates/sim`）

```bash
OWN_REMEASURE_OUT=docs/measurements/own_m0.json cargo test -p frontline-sim --features test-util --test own_remeasure -- --nocapture
cargo test -p frontline-sim --features test-util --test ref_regression      # 已 calibrated/locked 字段的回归（读 /data 的 _meta.ref）
python3 -m ref_analysis own-table docs/measurements/own_m0.json > docs/measurements/own_m0.md
```
`own_remeasure` 用与录像法相同的**定义**测我方：按下换弹到可再开火的 tick 数、最快连点的射击间隔、枪口到 200/300 m 的飞行时间 `d/t`、
稳态位移速度、静止到 95 % 速度的时间、`ceil(生命 / damage(部位, 距离))` 等；M0 sim 里不存在的功能（匍匐、翻越、切枪、晃动/屏息、逐发装填、FOV）
一律报 `unavailable` 并写明原因，**不用 0 冒充**。`ref_regression` 的 `_meta.ref` 块约定见该文件头注释。
