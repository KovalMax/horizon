#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
project="horizon-browser-$$"
mode=${1:-}
export HORIZON_BROWSER_RESULTS="${HORIZON_BROWSER_RESULTS:-$(pwd)/.local/browser-results}"
mkdir -p "$HORIZON_BROWSER_RESULTS"
HORIZON_BROWSER_RESULTS=$(cd "$HORIZON_BROWSER_RESULTS" && pwd)
certs=$(mktemp -d "$HORIZON_BROWSER_RESULTS/certs.XXXXXXXX")
export HORIZON_BROWSER_CERTS="$certs"
compose() {
  if [ "$mode" = stack ] && [ -n "${HORIZON_CHECK_API_IMAGE:-}" ]; then
    docker compose -p "$project" -f infra/compose.browser.yaml -f infra/compose.api-image.yaml "$@"
  else
    docker compose -p "$project" -f infra/compose.browser.yaml "$@"
  fi
}
cleanup() {
  compose down --remove-orphans --rmi local
  rm -rf "$certs"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
case "$mode" in
  deterministic)
    compose run --rm --no-deps --build browser npm run test:e2e -- --output /results/deterministic
    ;;
  stack)
    HORIZON_CERT_DIR="$certs" sh infra/scripts/local-certs.sh >/dev/null
    openssl x509 -in "$certs/horizon.pem" -pubkey -noout > "$certs/public.pem"
    openssl pkey -pubin -in "$certs/public.pem" -outform DER > "$certs/public.der"
    HORIZON_BROWSER_TLS_SPKI=$(openssl dgst -sha256 -binary "$certs/public.der" | openssl base64 -A)
    export HORIZON_BROWSER_TLS_SPKI
    if [ -n "${HORIZON_CHECK_API_IMAGE:-}" ]; then
      compose build stack-browser web
    else
      compose build stack-browser api web
    fi
    compose up --no-build --detach --wait traefik api web
    compose run --rm --no-deps stack-browser
    ;;
  *)
    echo "Usage: $0 deterministic|stack" >&2
    exit 2
    ;;
esac
