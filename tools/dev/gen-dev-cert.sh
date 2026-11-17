#!/usr/bin/env sh
# Development certificate for the WebTransport listener (M0-017). See DEVELOPING.md.
# usage: tools/dev/gen-dev-cert.sh [--out DIR] [--days N<=13]
set -eu
cd "$(dirname "$0")/../.."
exec cargo run -q -p frontline-server --bin gen_dev_cert -- "$@"
