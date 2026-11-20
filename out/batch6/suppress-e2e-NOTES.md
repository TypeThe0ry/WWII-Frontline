# B6 suppress-e2e · E1-13

- 完成真实战术服、WASM、生产 HUD 及原生 WebAudio 联验。权威 C2 压制改变 HUD 和音频参数。权威 C4 驱动倒地、救起、再倒地、阵亡、HQ 复生。既有 C4 roster flags 使用 `ALIVE=0x20`。
- `TMPDIR=<本批临时目录> scripts/suppress-e2e.sh`。无头 Chromium SwiftShader，普通 WS 1/1（1.0 min）、私有回环 netem 5% 1/1（1.3 min）。实际丢弃 1,061/20,958 包（5.06%）。C2/HUD 强度两轮均为 0.17647/0.14118，截止频率为 12,835.06/13,533.10 Hz，增益为 0.84929/0.86601，两轮恢复至 20,000 Hz/1.0。工作站按 PI-109 为非参考机。
- `CARGO_BUILD_JOBS=8 scripts/local-ci.sh rust lint` 通过。`ts` 741 + 8 用例通过，`FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh determinism` 三个种子均 18,000/18,000 tick 跨 native/WASM 一致。`node tools/ci/check-acceptance.mjs` 发现 0 个报告问题和 0 个 open task。GitHub Actions 未使用。
- 使用程序生成的 FCOL 场地，夹具单次发出尚未接线的 GADGET 位。正式地图、参考机、受信任证书及线上部署、真人录像、实素材音画未测。待 Fable 设计。报告见 `reports/m1-suppress-e2e.md`，遗留见 PI-B6-suppress-e2e-1 及 H-017/021/027/053。
