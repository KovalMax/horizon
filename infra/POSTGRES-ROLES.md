# PostgreSQL runtime and migration roles

Production runs the API as `horizon_runtime`. The `migrate` service runs on the
database network only, with the existing `POSTGRES_USER` / `POSTGRES_PASSWORD`
schema owner. Those credentials are no longer passed to the serving API.
This change does not transfer ownership or demote the original PostgreSQL
bootstrap role. Local development and checks keep their disposable login.

## Staging and first rollout

Do this before releasing the new Compose file. Back up the database, confirm
restore on staging, and schedule a restart for the PostgreSQL minor upgrade.
Use the existing Horizon database on the existing volume.

1. On a new database only, initialize the schema using the one-shot service
   (`docker compose ... run --rm migrate`). The Compose configuration requires
   the new runtime password even for a one-shot command, so set it beforehand.
2. Connect to the Horizon database as the existing migration login in `psql`.
   Create a dedicated role:

   ```sql
   CREATE ROLE horizon_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
       NOREPLICATION NOBYPASSRLS;
   ```

   Set its password interactively with `\password horizon_runtime`. Store the
   matching value as `POSTGRES_RUNTIME_PASSWORD` in the host's
   `.env.production`. Do not add the secret to Git or pass it on a command line.
3. In the same `psql` session, apply `infra/postgres-runtime-grants.sql` from a
   trusted copy of the release checkout (`\i /path/to/postgres-runtime-grants.sql`).
   The script grants application row access, not table ownership or access
   to SQLx's migration history. Apply it as the account that will create
   future tables and sequences. If you change the migration login or recreate
   `_sqlx_migrations`, recheck the grants before starting the API.
4. Rehearse migration, readiness, sign-in and reminder delivery on staging.
   Verify the running API uses `horizon_runtime`, and verify the migration
   login is absent from the serving container. Don't print container
   environment values when recording the result.

Keep both credentials on the host during application rollback. The previous
application image can use the runtime role under the new Compose
configuration. Rolling back an application image does not undo schema
migrations, role grants, or the database image change.
