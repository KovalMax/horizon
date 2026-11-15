#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
project="horizon-release-checks-$$"
compose() { docker compose -p "$project" -f compose.checks.yaml "$@"; }
cleanup() { compose down --remove-orphans; }
trap cleanup EXIT

compose build api web
compose up -d --wait db
compose run --rm --no-deps api cargo test --locked
compose run --rm --no-deps api cargo run --locked -- migrate
compose run --rm --no-deps api cargo run --locked -- migrate
compose run --rm --no-deps web npm test
compose run --rm --no-deps web npm run build

IMAGE_TAG=release-check \
ACME_EMAIL=ci@example.invalid \
POSTGRES_DB=horizon_check \
POSTGRES_USER=horizon_check \
POSTGRES_PASSWORD=ci-only-password \
AUTH_JWT_SECRET=ci-only-secret \
AUTH_JWT_ISSUER=https://horizon.false.tel \
AUTH_PUBLIC_URL=https://horizon.false.tel/auth/complete \
SMTP_HOST=localhost \
SMTP_FROM='Horizon <no-reply@example.invalid>' \
  docker compose -f infra/compose.prod.yaml config --quiet
