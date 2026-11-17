# 字体子集化流程（UI_SPEC §2.3；实现归 M1-053/M1-054）

本文只描述流程，不含实现代码。许可证登记见 `LICENSES.md` §3（`AST-V-FNT-UI-01`、`AST-V-FNT-004-01..03`）。

## 前置：许可证约束（OFL 1.1，FAQ 2.2/2.6/2.7）
- 子集化与有损格式转换属 Modified Version，**不得继续使用 Reserved Font Name**。
- Barlow Semi Condensed：无 RFN，可保留原名。
- Noto Sans SC：RFN 为 `Source`（Adobe），族名不含该词；子集产物 name 表不得出现 `Source`。
- IBM Plex Sans / Mono：RFN 为 `Plex`。在负责人对 `AST-V-FNT-004-01/02`（PENDING）作出决定前，**不得把子集化后的 Plex 产物入库**。若采用改名方案：用 fonttools 把 name 表（ID 1/4/6/16）中的 Plex 字样替换为新族名，CSS 同步改名。
- 产物目录须随附对应 `licenses/texts/*.txt` 许可证原文。

## 输入字符集
由构建期脚本生成（M1-053），来源：
1. i18n 文案文件（zh-CN）中出现的全部字符（自动抽取）；
2. GB2312 一级 + 二级汉字；
3. 游戏词表（装备、地图、术语）；
4. 常用中英文标点与全角符号。
启动页词表（启动页 i18n 键中出现的字符）单独导出为 `boot` 字符集。拉丁字体使用单独的 Basic Latin + Latin-1 + 数字符号集。

## 子集化命令（fonttools）
```
pip install fonttools brotli
pyftsubset NotoSansSC-VF.ttf --text-file=chars_<slice>.txt \
  --flavor=woff2 --layout-features='*' --no-hinting \
  --output-file=dist/fonts/noto-sans-sc-<slice>.woff2
```
- 可变字体：保留 wght 轴，限定 400-700：先 `fonttools varLib.instancer NotoSansSC-VF.ttf wght=400:700 -o NotoSansSC-400-700.ttf` 再子集化；也可按 400/500/700 三个静态实例各自子集化（UI_SPEC 允许两种）。
- 拉丁字体同样用 `pyftsubset --flavor=woff2`，每个字重一个文件。

## 切片规则（≥ 4 片）
1. `boot`：启动页词表字符，首屏唯一加载的中文片；
2. `core`：GB2312 一级常用汉字 + 标点；
3. `ext`：GB2312 二级汉字；
4. `game`：游戏词表中未被前三片覆盖的字符；
每片字符集互斥（后片 = 目标集合 − 已分配）。可再拆分，片数 ≥ 4。每片在 CSS 中以 `unicode-range` 声明（由构建脚本从字符集生成，写成 `U+XXXX-YYYY` 区间列表）。

## CSS 约定
- 每个 `@font-face` 使用 `font-display: swap`，`src` 指向同源自托管的 `.woff2`，不用外链 CDN。
- 族名与字重对应 UI_SPEC §2.3；`font-family` 回退链见 UI_SPEC §2.3 要点。
- 仅 `boot` 片与拉丁字体用 `<link rel=preload>`；其余片按 unicode-range 惰性加载。

## 预算检查（进入主菜单前 ≤ 400 KB，br 后，【待调参】）
```
for f in dist/fonts/*.woff2; do brotli -q 11 -c "$f" | wc -c; done | awk '{s+=$1} END {print s " bytes"; exit (s>409600)}'
```
只对进入主菜单前会加载的文件（`boot` 片 + 拉丁字体各字重）求和；退出码非 0 即超预算。

## 输入体积参考（已实测）
- google/fonts `ofl/notosanssc/NotoSansSC[wght].ttf`：17,772,300 字节（2026-11-17 下载量）。
- 其余字体与子集后体积本任务未测，由 M1-053 实测。
