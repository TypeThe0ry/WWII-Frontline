#!/usr/bin/env bash
# MILESTONES §2.4. Run after pnpm install --frozen-lockfile (see DEVELOPING.md).
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
repo=$PWD
report="$repo/reports/m1-demo-run.md"
base=$(git rev-parse --short HEAD)
scratch=${FRONTLINE_DEMO_TMPDIR:-${TMPDIR:-/tmp}}
if [[ -z ${FRONTLINE_DEMO_TMPDIR:-} && -d /data/scratch && -w /data/scratch ]]; then scratch=/data/scratch; fi
run=$(mktemp -d "$(cd "$scratch" && pwd)/frontline-demo-m1.XXXXXXXX")
export TMPDIR=$run FRONTLINE_M1_DEMO_DIR=$run

steps=(
  '启动与主菜单'
  '服务器列表、游客入局、HQ 部署'
  '移动、射击与 Garrison 重生'
  'Support 补给与补弹'
  'Medic 与压制'
  'Sherman 驾驶'
  '据点规则'
  '指标、素材报告与结算'
)
human=(
  '未测（需人）：预发布域名、受信任 HTTPS 与正式包下载过程（H-017）。'
  '未测（需人）：20 人测试服、地图画布键鼠部署、倒计时内 Carentan 实图预取演示（H-054/H-057）。'
  '未测（需人）：姿态/翻越、Thompson 与 Garand 双枪型、全息绿/红和实图真人队友操作（H-057）。'
  '未测（需人）：实图真人搬运、放置与补弹（H-057）。'
  '未测（需人）：浏览器内真人 Medic 救援与压制音效/屏幕效果（H-057）。'
  '未测（需人）：三人乘员、实图路线、游戏内双方交战（H-054/H-057）。'
  '未测（需人）：L1a/L2a/L3a 真人拉锯及战术地图显示（H-054/H-057）。'
  '未测（需人）：预发布监控看板及真人对局自然结束后的结算展示（H-057）。'
)

finish() {
  local status=$? output="$run/report.md" n evidence count=0
  trap - EXIT
  {
    printf '# M1 八步演示运行记录\n\n'
    printf -- '- 运行日期 %s。检出提交 `%s`。命令 `scripts/demo-m1.sh`。\n' "$(date +%F)" "$base"
    printf -- '- PI-109 工作站（非参考机）。本机 HTTP/WS、无头 Chromium 软件 WebGL2，使用生成的战术碰撞图和独立 M0 驾驶夹具。\n'
    printf -- '- 脚本退出码 `%d`。下表的 PASS 对应本地子项。\n\n' "$status"
    printf '| §2.4 | 演示步骤 | 本地检查 | 待人工演示 |\n|---|---|---|---|\n'
    for n in "${!steps[@]}"; do
      if [[ -s $run/step-$((n + 1)) ]]; then
        evidence="PASS — $(<"$run/step-$((n + 1))")"
        count=$((count + 1))
      else
        evidence='未测（未取得通过证据，见运行日志）'
      fi
      printf '[%d/8] %s — %s\n' "$((n + 1))" "${steps[n]}" "$evidence" >&2
      printf '      %s\n' "${human[n]}" >&2
      printf '| %d | %s | %s | %s |\n' "$((n + 1))" "${steps[n]}" "$evidence" "${human[n]}"
    done
    printf '\n本地子项 %d/8 取得通过证据。完整 M1 退出标准见 [E1-1～E1-17](m1-acceptance.md)。现场材料见 [STATUS](../tasks/STATUS.md)、[占位决定](m1-placeholders.md) 和 [构建缺失清单](../assets/manifests/missing-report.json)。\n' "$count"
  } >"$output"
  if ! cp "$output" "$report"; then status=1; fi
  rm -rf -- "$run"
  printf '报告 %s\n' "$report" >&2
  exit "$status"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

cargo build --locked -p frontline-gateway -p frontline-server
wasm-pack build crates/sim-wasm --target web --profile wasm-release
pnpm -C client build
core=$(node tools/ci/core-size.mjs --dist client/dist --max 8000000 --report "$run/core.json")
[[ -s client/dist/wasm/frontline_sim_wasm_bg.wasm ]]

status=0
if ! pnpm -C client exec playwright test -c playwright.garrison.config.ts --grep 'over ws$' --output "$run/playwright"; then
  status=1
fi
if [[ -s $run/step-1 ]]; then printf ' 生产 core（含 WASM）%s。' "$core" >> "$run/step-1"; fi

if cargo test --locked -p frontline-sim --features test-util --test health_m1 --test suppression_m1; then
  printf 'sim 夹具通过受伤/倒地/救起和压制规则检查。' > "$run/step-5"
else
  status=1
fi
if FRONTLINE_REQUIRE_SERVER=1 FRONTLINE_SERVER_BIN="$repo/target/debug/frontline-server" \
    cargo test --locked -p frontline-headless-client --test vehicle_flow -- --nocapture 2>&1 | tee "$run/vehicle.log"; then
  vehicle=$(awk '/^driver seat 0 on vehicle/ { print; exit }' "$run/vehicle.log")
  if [[ -n $vehicle ]]; then
    printf '独立 M0 WS 驾驶，权威入座 → 行驶 → 离座。%s。' "$vehicle" > "$run/step-6"
  else
    status=1
  fi
else
  status=1
fi
if cargo test --locked -p frontline-sim --features test-util --test match_state_m1; then
  printf 'sim 对局夹具通过预热、据点推进与胜负条件检查。' > "$run/step-7"
else
  status=1
fi
for n in {1..8}; do [[ -s $run/step-$n ]] || status=1; done
exit "$status"
