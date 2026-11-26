#!/bin/sh
set -eu

# Accept the isolated Compose command as arguments so local and CI checks
# exercise the same production grant SQL without touching a deployed stack.
"$@" run --rm --no-deps api target/debug/horizon-api migrate
"$@" exec -T db sh -ec '
    exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
      -c "CREATE ROLE horizon_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS" \
      -f -
' < infra/postgres-runtime-grants.sql
"$@" exec -T db sh -ec '
    exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f -
' < infra/scripts/check-postgres-runtime.sql
