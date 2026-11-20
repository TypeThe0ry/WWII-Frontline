| 浏览器 (Playwright 构建) | WT | WS | 回退（删除 WebTransport） | 回退（WT 端口关闭） | 备注 |
| --- | --- | --- | --- | --- | --- |
| chromium 153.0.8010.12 | 通过（200 快照/10 s，RTT 中位 1.1 ms） | 通过（151 快照/10 s，RTT 中位 0.7 ms） | 通过（回退 WS，7 ms） | 通过（回退 WS，25 ms） | headless Chromium；C2 来源 server |
| firefox 155.0 | 通过（201 快照/10 s，RTT 中位 1.5 ms） | 通过（150 快照/10 s，RTT 中位 1.4 ms） | 通过（回退 WS，8 ms） | 未设计（任务仅要求 Chromium） | Playwright 打补丁的 Firefox 构建；C2 来源 server |
| webkit 26.6 | 失败：无 WebTransport（0 快照） | 通过（150 快照/10 s，RTT 中位 0.5 ms） | 通过（回退 WS，5 ms） | 未设计（任务仅要求 Chromium） | WebKit 构建 ≠ Safari 正式版；C2 来源 server |
