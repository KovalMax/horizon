#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
project="horizon-checks-$$"
compose() { docker compose -p "$project" -f infra/compose.checks.yaml "$@"; }
cleanup() { compose down --remove-orphans; }
trap cleanup EXIT
compose build api web
compose up -d --wait db
compose run --rm --no-deps api cargo fmt --all -- --check
compose run --rm --no-deps api cargo clippy --locked --all-targets -- -D warnings
compose run --rm --no-deps api cargo test --locked
sh infra/scripts/check-postgres-runtime.sh docker compose -p "$project" -f infra/compose.checks.yaml
compose run --rm --no-deps web npm run format:check
compose run --rm --no-deps web npm run build
compose run --rm --no-deps web npm test
