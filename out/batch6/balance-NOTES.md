# B6-balance：M2-018

- 已做：确定性阵营准入/强制调队、TK、投票、AFK 与重连状态；接入 server/C0/C4、WASM 与客户端状态。协议 v5，网关票据 v2，`DATA_HASH=0x7ff50022cf048e0b`，无新依赖/素材。
- 没做：UI 待 Fable 设计。语音、正式地图轮转、Commander 席位、Party/局间产品接线未做；计分归 M2-017、视觉归 M2-043。参考机、真人输入回放与预发布受信任 TLS 未测。M2-018 `partial`，不改变 M1 退出判定。
- 证据：见 [逐项验收与命令](../../reports/m2-server-rules.md)、PI-B6-balance-1、H-017/H-021/H-054/H-060。工作站 PI-109 非参考机；2 个 30 客户端回环案，TK 案 1 次原谅/5 次处罚/1 次断线，其他 29 个会话在线；合成 AFK 0/20 误判。
- `CARGO_BUILD_JOBS=8 cargo bench --locked -p frontline-sim --bench serverrules_tick`：100 人、50×1,000 tick；纯规则 P50/P95 2.94/3.60 µs，含输入 P50/P95 8.09/10.99 µs，预算 50 µs/tick；不含 World 和网络。
