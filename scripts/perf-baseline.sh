#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p .local/perf-baseline
output=$(mktemp -d "$(pwd)/.local/perf-baseline/run.XXXXXXXX")
certs=$(mktemp -d "$(pwd)/.local/perf-baseline/certs.XXXXXXXX")
project="horizon-perf-$$"
export HORIZON_BROWSER_RESULTS="$output"
export HORIZON_BROWSER_CERTS="$certs"
export PERF_REVISION="$(git rev-parse HEAD)$(if [ -n "$(git status --porcelain)" ]; then printf %s '-dirty'; fi)"
compose() { docker compose -p "$project" -f compose.browser.yaml -f compose.perf.yaml "$@"; }
cleanup() {
  compose down --remove-orphans --rmi local
  rm -rf "$certs"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
psql() { compose exec -T db psql -X -v ON_ERROR_STOP=1 -U horizon_browser -d horizon_browser "$@"; }

printf 'Performance artifacts: %s\n' "$output"
HORIZON_CERT_DIR="$certs" sh scripts/local-certs.sh >/dev/null
compose build api web perf
compose up --detach --wait db mailpit web traefik
compose run --rm --no-deps api migrate
psql < scripts/perf/seed.sql > "$output/fixtures.txt"
psql -c 'SELECT pg_stat_statements_reset()' >/dev/null
compose up --detach --no-deps api
compose run --rm --no-deps perf
compose logs --no-color --no-log-prefix api | awk '/\[horizon_api::calendar_profile\]/' > "$output/calendar-profile.log"
psql -At -v date="$(cat "$output/day")" \
  -v mixed_start="$(cat "$output/mixed-start")" -v mixed_end="$(cat "$output/mixed-end")" \
  < scripts/perf/report.sql > "$output/postgres.json"
psql -At -v date="$(cat "$output/day")" < "$output/explain.sql" > "$output/query-plans.txt"
compose run --rm --no-deps perf node /perf/render-report.mjs
printf 'Baseline complete: %s\n' "$output"
