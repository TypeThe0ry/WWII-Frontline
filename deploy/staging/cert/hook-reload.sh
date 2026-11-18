#!/usr/bin/env sh
# certbot deploy hook: copy the fresh cert into CERT_DIR and SIGHUP the server container.
set -eu
: "${CERT_DIR:?}"
cp "$RENEWED_LINEAGE/fullchain.pem" "$CERT_DIR/fullchain.pem"
cp "$RENEWED_LINEAGE/privkey.pem" "$CERT_DIR/privkey.pem"
docker kill --signal=HUP "$(docker compose ps -q server)"
