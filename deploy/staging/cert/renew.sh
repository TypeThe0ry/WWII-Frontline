#!/usr/bin/env sh
# Renew the match-host certificate and tell `server` to reload it (SIGHUP) — no restart, running matches continue.
# Requires: certbot, ACME_EMAIL, MATCH_HOST_DOMAIN, CERT_DIR. Cron/systemd timer: twice a day.
set -eu
: "${MATCH_HOST_DOMAIN:?}" "${ACME_EMAIL:?}" "${CERT_DIR:?}"
certbot certonly --standalone -d "$MATCH_HOST_DOMAIN" -m "$ACME_EMAIL" --agree-tos --non-interactive \
  --deploy-hook "$(dirname "$0")/hook-reload.sh"
