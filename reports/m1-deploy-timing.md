# E1-10：部署“点击到可操作”耗时与 `R_near` 预取完成率（B6-deploytiming）

2026-11-20，EPYC 7402 工作站（**PI-109，非参考机**，不替代 RTX 3060 / Iris Xe 参考机 H-021）。Playwright 无头 Chromium `153.0.8010.12`（SwiftShader WebGL2 软件渲染，未占用 GPU0），真实本机 `frontline-gateway`、`frontline-server`（`--map carentan/m1 --mode assault`，真实 Carentan M1 380 块解压 FCOL 与战术规则）及静态/HTTP 206 Range/代理服托管生产构建 `client/dist` 与 Carentan M1 30 个地形 `.fpak`（共 380 块、5,714,050 B）。原始数据见 `reports/m1-deploy-timing.json`。

## 测量口径与环境

- **带宽与延迟限制**：通过 CDP `Network.emulateNetworkConditions` 施加两档受限网络：
  - `20 Mbps / 50 ms RTT`（下行 20 Mbps、上行 10 Mbps、延迟 50 ms）
  - `5 Mbps / 100 ms RTT`（下行 5 Mbps、上行 2 Mbps、延迟 100 ms）
- **冷/热缓存（OPFS）**：每档冷/热各 5 次（共 20 次实测样本）。
  - 冷缓存：每次独立 `BrowserContext`、禁用浏览器 HTTP 缓存并清空 OPFS。
  - 热缓存：同 `BrowserContext` 先跑 1 次预热（不计入样本）填充 OPFS 与 HTTP 缓存，随后每次新开独立 `Page`（JS 内存 LRU 仅 2 MiB 且每次为空），强制从持久化 OPFS `FileSystemSyncAccessHandle`（`opfsSync=true`）读取全部 `.fpak` 条目。
- **端到端链路**：
  1. 页面打开生产 `client/dist` → `Boot` WebGL2 能力探测 → 点击 `Click to continue` → `Menu` 调真实 `/v1/servers` → 点击 `Quick join`。
  2. 点击 `Join with automatic assignment`（**计时起点**）→ 真实 `/v1/guest` + `/v1/servers/{id}/ticket` → `bootWorld`（WebGL2 渲染器 + `sim-wasm` + 真实 WS `Hello`/`Welcome` + `Combat HUD` 挂载）。
  3. 同步发起真实 C0 `DeployRequest`（HQ 出生、Rifleman）等待权威 C0 `DeployResult` 与 C2 `health > 0`（并确认后续一帧 C2 存活），**并行**在 Carentan M1 切片内陆驻点候选坐标 `(650, 1470)`（chunk `(10, 22)`，`R_NEAR = 4` 环 9×9 = **81/81 块已烘焙**）运行真实 `ChunkStreamer.prefetchAround(..., { ring: 4 })`（经 `AssetLoader` + `CacheLayer(opfsSyncBackend)` + `fpakChunkSource` + Worker 解码全部 81 块视觉 FCHK 与 81 块碰撞 FCOL，共 162 条 `.fpak` 条目）。
  4. **点击到可操作（`clickToPlayableMs`）** = `joinToHudMs + max(spawnAliveMs, prefetchMs)`，不含测量脚本本身的瞬时注入时间。

> **切片边界说明**：Carentan M1 为 1/3 地图切片（380 块），US HQ `(476, 1600)` 位于切片边缘，`R_NEAR=4` 网格 81 块中仅 42 块（51.9%）在切片内烘焙；因此预取率以切片内完整 81/81 覆盖的 US 驻点候选 `(650, 1470)` 实测。另外，为绕开本机单 IP `10 req/min` 的网关限速而不改动生产限流代码，测量代理在回环上按轮次轮换源地址 `127.0.1.x`（与 `headless-client` 和 `garrison-loop.spec.ts` 一致）。

## 实测分布（各 5 次，PI-109 非参考机）

| 网络档位 | 缓存 | 样本 | 点击到可操作 P50 / P95 / max | 页面加载到可操作 P50 / max | Join→HUD P50 | C0 回执 P50 | C2 出生 P50 | `R_near` 预取 P50 | `R_near` 完成率 | `.fpak` 请求 / 字节 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 20 Mbps / 50 ms RTT | 冷 (OPFS 空) | 5 | **6,249.2** / 6,484.8 / 6,484.8 ms | 6,777.3 / 7,236.3 ms | 2,893.9 ms | 24.1 ms | 126.4 ms | 3,225.9 ms | **81/81 (100%)** | 189 / 1,095,581 B |
| 20 Mbps / 50 ms RTT | 热 (OPFS) | 5 | **1,419.4** / 1,426.7 / 1,426.7 ms | 1,697.4 / 1,721.4 ms | 892.6 ms | 20.1 ms | 127.2 ms | 531.3 ms | **81/81 (100%)** | **0 / 0 B** |
| 5 Mbps / 100 ms RTT | 冷 (OPFS 空) | 5 | **12,850.8** / 12,997.6 / 12,997.6 ms | 14,205.0 / 14,357.1 ms | 7,365.9 ms | 27.9 ms | 115.7 ms | 5,480.3 ms | **81/81 (100%)** | 189 / 1,095,581 B |
| 5 Mbps / 100 ms RTT | 热 (OPFS) | 5 | **1,832.6** / 1,863.4 / 1,863.4 ms | 2,151.1 / 2,162.8 ms | 1,344.5 ms | 30.0 ms | 125.4 ms | 498.9 ms | **81/81 (100%)** | **0 / 0 B** |

### 关键结论

- **点击到可操作 ≤ 15 s**：四组全部满足工作站口径——最慢档（5 Mbps / 100 ms RTT 冷缓存）P50 **12.85 s**、max **13.00 s**（含首次页面加载全程 max 14.36 s）；20 Mbps / 50 ms RTT 冷缓存 P50 **6.25 s**、热缓存 P50 **1.42 s**。
- **`R_near` 预取完成率 ≥ 95%**：20/20 轮全部 **81/81（100.0%）**，视觉与碰撞块均 81/81 驻留，0 失败。
- **OPFS 热缓存效果**：10/10 轮热缓存 `.fpak` HTTP 请求数均为 **0（0 B）**，预取阶段耗时较冷缓存分别下降 **83.5%**（20 Mbps：3,225.9 → 531.3 ms）和 **90.9%**（5 Mbps：5,480.3 → 498.9 ms）。
- **发现并修复的接线缺口**：`client/src/render/assets/manifest.ts` 的 `ManifestIndex` 原未索引 `assets/manifests/terrain_carentan.json` 中的 `chunk` 字段（仅索引 `lods`/`collision`/`textures`/`audio`/`meta`），已补充 `chunk?: EntryRef` 与对应单测（见 `PI-B6-deploytiming-1`）。

## 复跑命令

```sh
CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -p frontline-gateway -p frontline-server
CARGO_BUILD_JOBS=8 wasm-pack build crates/sim-wasm --target web --profile wasm-release
pnpm -C client build
node scripts/measure-deploy.mjs --runs 5 --json reports/m1-deploy-timing.json
```

## 边界与未测项

- 本报告为 EPYC 7402 工作站 + SwiftShader 软件渲染实测（**PI-109，非参考机**），不替代 RTX 3060 / Iris Xe 参考机（H-021）。
- 仅包含 Carentan M1 地形草稿 `.fpak`（5.71 MB），不含 M1-064 真实美术/音频整包（≤ 150 MB 预算下冷缓存时间需在素材入包后复测）。
- 未在 M1-067 预发布公网域名 / HTTPS / CDN / 受信任证书（H-015/016/017/020）下实测；主游戏页面尚未把完整部署选点界面直接接入 `ChunkStreamer`，因此 E1-10 保持 `partial`。
