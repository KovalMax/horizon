#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p .local
certs=$(mktemp -d "$(pwd)/.local/browser-certs.XXXXXXXX")
project="horizon-browser-$$"
export HORIZON_BROWSER_CERTS="$certs"
export HORIZON_BROWSER_RESULTS="${HORIZON_BROWSER_RESULTS:-$(pwd)/.local/browser-results}"
mkdir -p "$HORIZON_BROWSER_RESULTS"
compose() { docker compose -p "$project" -f compose.browser.yaml "$@"; }
cleanup() {
  compose down --remove-orphans --rmi local
  rm -rf "$certs"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
case "${1:-}" in
  deterministic)
    compose run --rm --no-deps --build browser npm run test:e2e -- --output /results/deterministic
    ;;
  stack)
    HORIZON_CERT_DIR="$certs" sh scripts/local-certs.sh >/dev/null
    compose build stack-browser api web
    compose up --detach --wait traefik api web
    compose run --rm --no-deps stack-browser
    ;;
  *)
    echo "Usage: $0 deterministic|stack" >&2
    exit 2
    ;;
esac
