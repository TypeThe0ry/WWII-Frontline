# B6-mines landall 复验

- 基线：`d537011`（M2-009 resources，协议 v12）。rebase 冲突均保留双方数据：`emplacement` 与 ordnance modules、economy/resource-node/ordnance 校验和、`ResourceNodeDef` 与 `OrdnanceRulesDef` typed validation。
- 最小代码修复：修复 `mines_tick` bench 对 mine 模块的路径；为数据驱动 `Default` 与序列化 `hash` 名称添加局部 clippy 说明；整理 Rust 格式。未屏蔽测试、未改预算。
- 命令：`CARGO_BUILD_JOBS=8 scripts/local-ci.sh lint`，通过。
- 命令：`CARGO_BUILD_JOBS=8 FRONTLINE_DETERMINISM_STRICT=1 scripts/local-ci.sh rust determinism data`，三组通过。严格 native/WASM 3 种子各 18,000 tick 哈希一致，最大位置误差 0 m；data 92 文件 0 错误。
- 定向命令：`cargo test -p frontline-sim --features test-util mine::`、`explosive::`、`weapon::launcher`、`projectile::heat`，通过。
- 状态为 `partial`：独立规则 API 未接入 World tick、输入、服务端事件或正式 weapons 数据；M2-012/013 前置、H-021 参考机和 H-060 参考作品/真人校准未测。PI-109 工作站数据不作为参考机结果。详见 `reports/m2-mines.md` 与 PI-B6-landall-mines-1。
