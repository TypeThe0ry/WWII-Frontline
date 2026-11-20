# UI 字体子集化（M1-053 / UI_SPEC §2.3）

`tools/font-subset/subset.py` 离线读取 TTF 和当前 worktree 的 `LICENSES.md`。不下载字体，也不按文件名或系统已安装的同族字体推定许可。输出为独立静态目录，不进 `.fpak`。不要把旧的 `--fetch` 产物当作获准发布的输入。

## 台账与产物

| 用途 | 族策略 / 原档行 | 本地状态（2026-11-20） |
|---|---|---|
| Barlow Semi Condensed 500/600 | `AST-V-FNT-UI-01/02` | 01 保留原族策略并登记 500；02 登记 600。共用同一份 OFL。 |
| Frontline UI Sans / Mono | `AST-V-FNT-004-01/02` | PI-009 方案 A 已落实，两行均 OK；`CONVERT,SUBSET,RENAME`。 |
| Noto Sans SC 400–700 | `AST-V-FNT-004-03` / `AST-V-FNT-004-04` | 03 保留族策略，04 为原档行；均 OK，允许 `CONVERT,SUBSET`。 |

完整构建要求族策略与唯一的逐文件登记均为 `OK`、`SIL OFL 1.1`，有 `CONVERT,SUBSET`（Plex 另需 `RENAME`），许可文本路径一致，TTF 和许可全文字节分别与台账的 `SHA-256` 和 `OFL_SHA256` 相符。未满足则退出 2，不生成输出。五个文件均来自 `google/fonts@cc5661d43cd4f9aad5567033a8e15ca28dc35c86`；每个文件的固定 raw URL 和逻辑 `assets/incoming/` 路径见台账，实际工作副本仅暂存 scratch。该 revision 的四份 `OFL.txt` 均与已有归档逐字节一致，没有做换行或其他规范化。

每个来源附原样复制的 OFL 全文（包括版权与 RFN 声明）。按 PI-009，Plex 派生字体使用 `Frontline UI Sans` / `Frontline UI Mono` 的内部名称与 CSS；法定版权和许可记录不作改名。Noto 的 name 表保留 Adobe 版权，但不含 `Source`；完整的 RFN 声明保留在随附文本。输出后重新打开 WOFF2 检查 cmap 和保留名。

## 字符与加载

构建从 `client/src/ui/i18n/{en,zh-Hans}.json` 的值提取词表，不修改语言包。拉丁字体单独生成。中文四片互斥，每片剔除先前已分配的字符：

| 分片 | 内容 |
|---|---|
| `cjk0` | `ui.boot.*` 及启动页复用的 `ui.menu.retry` 中需要中文字体的字符，包括用到的中英文共享标点。启动页引用新的非 boot 键时需同步选择器。 |
| `cjk1` | 两语言中其余非拉丁词表和常用标点。词表生僻字不局限于 GB2312。 |
| `cjk2` | GB2312 一级（3,755 字）中尚未分配的字符。 |
| `cjk3` | GB2312 二级（3,008 字）中尚未分配的字符。 |

生成的 `fonts.css` 使用 `font-display: swap`，`unicode-range` 与实际 cmap 一致。`report.json` 记录每个输入、许可和两份语言包的 SHA-256，逐片列出 `shard`、`chars`、`bytes`（WOFF2）及 `br_bytes`（再次按 Brotli 质量 11 压缩）。构建期间文案发生改变会报错；提交验收数据前用下述测试再次对照当前词表。

完整包中，英文首屏预算为四个拉丁文件之和（≤204,800 B），中文为四个拉丁文件加 `cjk0`（≤409,600 B），均按逐文件 `br_bytes` 计。仅测字体载荷，不计 CSS、HTTP 头、系统后备字体或网络时序。浏览器按可见字符请求字体；不要预载全部分片。完整包就位后，应用入口可调用 `installUiFonts(document)` 安装同源 `/fonts/fonts.css` 并在销毁时解绑；样式表或 UI 字体加载失败时设置 `data-font-fallback`。此工具不修改应用入口或指定视觉样式。

## 工作站复核

从仓库根目录执行。将五个固定 raw URL 的原档以原文件名放在一个 scratch 目录，或复用此次留存的 `src`。需要 Python 3.10+、fontTools、Brotli；本机现有 `python3` 环境已具备（实测 4.61.1 / 1.2.0）。`--out` 必须不存在，避免旧字体、旧字集和旧许可混入新包。

```sh
src=/data/scratch/uilogic-fonts-full.F51ulU/src
run=$(mktemp -d /data/scratch/uilogic-fonts.XXXXXX)
python3 -B tools/font-subset/subset.py --src "$src" --out "$run/fonts"
FONT_CHECK_DIR="$run/fonts" python3 -B -m unittest discover -s tools/font-subset -v
pnpm -C client exec vitest run src/ui/fonts/loader.test.ts
```

仅诊断 Noto 时可改用 `--check-noto` 和 `NOTO_CHECK_DIR`，仍须相同的许可与哈希校验；单项报告不会给出全族预算。两种环境变量均未设置时，单测会明确跳过产物检查。

2026-11-20 全量样本：149 个 i18n 键，英文/中文目录 SHA-256 前缀 `1b6d9dbb` / `1f603995`。英文 4 片合计 55,114 B br；中文加 89 个码点的启动片（23,672 B br），共 78,786 B br。全部 8 片共 1,897,392 B WOFF2。此为构建期实测，不是浏览器网络测量；词表更新后以新 `report.json` 为准。
