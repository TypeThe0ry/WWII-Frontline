# docs/measurements —— 对标测量记录（REFERENCE_MEASUREMENTS §8）

**这里只放数字和文字。** 不放参考作品的录像、截图、音频、图像或其商标图形；记录里统一称"参考作品"。

| 路径 | 内容 | 状态 |
|---|---|---|
| `summary.csv` | 汇总（25 列，§8.2）。目前**只有表头**：参考作品的测量值一律"待测"，没有录像就没有行 | 待录像（H-011/H-025/H-026） |
| `_TEMPLATE.md` | 单项记录模板（§8.1 原样） | 就绪 |
| `{ID}.md` | 单项记录（武器类 `W-01.us_m1_garand.md`，H-01 带 `@50m`）。由 `python3 -m ref_analysis record` 生成 | 无（待录像） |
| `series/` | 序列型结果（后坐模式、飞行时间表……）`{ID}.csv`，列 `index_or_t, value, unit, rep` | 空 |
| `own_m0.json` / `own_m0.md` | **我方游戏**同法复测的批 M0 数值（`cargo test -p frontline-sim --test own_remeasure` 生成；`.md` 是同一份数据的表格） | 已生成 |
| `CAPTURE_GUIDE_M0.md` | 给人类负责人的一页清单：录什么、什么设置、放哪、跑哪条命令 | 就绪 |
| `raw/` | **录像私有存储（被 .gitignore 忽略，永不入库）**；也可放仓库外，用环境变量 `REF_CAPTURE_DIR` 指向 | 本地 |
| `M0-023/ M0-024/ M0-025/` | 其它任务（地形/植被/灰盒角色）的验收截图，**与对标测量无关**，见 PLAN_ISSUES PI-027 | 非本任务 |

## 录像私有存储与合规（§1.2/§1.3 摘要）

- 只做黑盒观测：不提取素材、不反编译、不读内存、不解析协议。
- 录像、抽帧 PNG、提取的 wav 只放私有存储（`docs/measurements/raw/`、`assets/incoming/ref-captures/` 已被忽略，`frames/`、`tmp/` 也忽略）；
  不得上传到公共或不受控的服务；传输方式由人决定（H-033）。
- 分析脚本只读取录像、只输出数字；`record.py` 拒绝把图片文件名写进记录。
- 参考作品更新后旧行标 `stale`（保留，不覆盖），重测新建一行并带新版本号。

## 常用命令

```bash
export PYTHONPATH=tools/ref-analysis
python3 -m ref_analysis validate     # summary.csv 合法性（25 列、枚举）
python3 -m ref_analysis status -v    # 批 M0 完成度（缺项退出码 1；当前 78 行全部缺）
python3 -m ref_analysis checklist    # 批 M0 全部行（也已写进 CAPTURE_GUIDE_M0.md）
```
