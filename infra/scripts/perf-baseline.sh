#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
results="${HORIZON_PERF_RESULTS:-$(pwd)/.local/perf-baseline}"
mkdir -p "$results"
results=$(cd "$results" && pwd)
output=$(mktemp -d "$results/run.XXXXXXXX")
certs=$(mktemp -d "$results/certs.XXXXXXXX")
project="horizon-perf-$$"
export HORIZON_BROWSER_RESULTS="$output"
export HORIZON_BROWSER_CERTS="$certs"
export PERF_REVISION="$(git rev-parse HEAD)$(if [ -n "$(git status --porcelain)" ]; then printf %s '-dirty'; fi)"
sampler_pid=
compose() { docker compose -p "$project" -f infra/compose.browser.yaml -f infra/compose.perf.yaml "$@"; }
cleanup() {
  if [ -n "$sampler_pid" ]; then
    kill "$sampler_pid" 2>/dev/null || true
    wait "$sampler_pid" 2>/dev/null || true
  fi
  compose down --remove-orphans --rmi local
  rm -rf "$certs"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
psql() { compose exec -T db psql -X -v ON_ERROR_STOP=1 -U horizon_browser -d horizon_browser "$@"; }

printf 'Performance artifacts: %s\n' "$output"
HORIZON_CERT_DIR="$certs" sh infra/scripts/local-certs.sh >/dev/null
compose build api web perf
compose up --detach --wait db mailpit web traefik
compose run --rm --no-deps api migrate
psql < infra/perf/seed.sql > "$output/fixtures.txt"
if [ "${PERF_REMINDER_BACKLOG:-0}" = 1 ]; then
  psql < infra/perf/reminder-backlog.sql >> "$output/fixtures.txt"
  psql -c "ALTER SYSTEM SET log_lock_waits = 'on'" >/dev/null
  psql -c "ALTER SYSTEM SET deadlock_timeout = '50ms'" >/dev/null
  psql -c 'SELECT pg_reload_conf()' >/dev/null
  psql -At -F, -f - < infra/perf/reminder-backlog-sample.sql > "$output/reminder-start.csv"
  { cat infra/perf/reminder-backlog-sample.sql; printf '\\watch i=0.25 c=1600\n'; } |
    psql -At -F, > "$output/reminder-samples.txt" &
  sampler_pid=$!
fi
psql -c 'SELECT pg_stat_statements_reset()' >/dev/null
compose up --detach --no-deps api
compose run --rm --no-deps perf
if [ "${PERF_REMINDER_BACKLOG:-0}" = 1 ]; then
  kill "$sampler_pid" 2>/dev/null || true
  wait "$sampler_pid" 2>/dev/null || true
  sampler_pid=
  psql -At -F, -f - < infra/perf/reminder-backlog-sample.sql > "$output/reminder-end.csv"
  compose logs --no-color --no-log-prefix api |
    awk '/\[horizon_api::reminder_backlog_profile\]/' > "$output/reminder-pool.log"
  compose logs --no-color --no-log-prefix db |
    awk '/still waiting for .* after [0-9.]+ ms|acquired .* after [0-9.]+ ms|deadlock detected/ { print }' |
    sed -E 's/^.*(still waiting for|acquired|deadlock detected).*/\1/' |
    sort | uniq -c > "$output/reminder-lock-log-counts.txt"
fi
compose logs --no-color --no-log-prefix api | awk '/\[horizon_api::calendar_profile\]/' > "$output/calendar-profile.log"
psql -At -v date="$(cat "$output/day")" \
  -v mixed_start="$(cat "$output/mixed-start")" -v mixed_end="$(cat "$output/mixed-end")" \
  < infra/perf/report.sql > "$output/postgres.json"
psql -At -v date="$(cat "$output/day")" < "$output/explain.sql" > "$output/query-plans.txt"
compose run --rm --no-deps perf node /perf/render-report.mjs
printf 'Baseline complete: %s\n' "$output"
