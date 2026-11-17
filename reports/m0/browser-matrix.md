| 浏览器 (Playwright 构建) | WT | WS | 回退（删除 WebTransport） | 回退（WT 端口关闭） | 备注 |
| --- | --- | --- | --- | --- | --- |
| chromium 153.0.8010.12 | 通过（201 快照/10 s，RTT 中位 0.8 ms） | 通过（150 快照/10 s，RTT 中位 0.9 ms） | 通过（回退 WS，6 ms） | 通过（回退 WS，15 ms） | headless Chromium；C2 来源 server |
| firefox n/a | 未验证（browser binary not installed (npx playwright install firefox)） | 未验证（browser binary not installed (npx playwright install firefox)） | 未验证（browser binary not installed (npx playwright install firefox)） | 未设计（任务仅要求 Chromium） | Playwright 打补丁的 Firefox 构建 |
| webkit n/a | 未验证（browser binary not installed (npx playwright install webkit)） | 未验证（browser binary not installed (npx playwright install webkit)） | 未验证（browser binary not installed (npx playwright install webkit)） | 未设计（任务仅要求 Chromium） | WebKit 构建 ≠ Safari 正式版 |
