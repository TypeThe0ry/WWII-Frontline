#!/usr/bin/env sh
# Prints days until the certificate expires; exits 1 when ≤ 14 (alert threshold, ARCHITECTURE §18.2).
set -eu
f="${1:?cert file}"
end=$(openssl x509 -enddate -noout -in "$f" | cut -d= -f2)
left=$(( ( $(date -d "$end" +%s) - $(date +%s) ) / 86400 ))
echo "$left"
[ "$left" -gt 14 ]
