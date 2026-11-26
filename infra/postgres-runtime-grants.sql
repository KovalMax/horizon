-- Run as the existing schema owner / migration login after the initial migration.
\set ON_ERROR_STOP on
BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT FROM pg_roles r
        WHERE r.rolname = 'horizon_runtime' AND r.rolcanlogin
          AND NOT (r.rolsuper OR r.rolcreatedb OR r.rolcreaterole
                   OR r.rolreplication OR r.rolbypassrls)
          AND NOT EXISTS (SELECT FROM pg_auth_members m WHERE m.member = r.oid)
    ) THEN
        RAISE EXCEPTION 'Create horizon_runtime as an unprivileged login with no role memberships first';
    END IF;
END $$;

-- The REVOKE on SQLx bookkeeping intentionally requires an
-- initialized database; run the initial migrations before applying grants.
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO horizon_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO horizon_runtime;
REVOKE ALL ON public._sqlx_migrations FROM PUBLIC, horizon_runtime;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO horizon_runtime;

-- Apply to objects created by the current migration login, not cluster-wide.
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO horizon_runtime;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE ON SEQUENCES TO horizon_runtime;

COMMIT;
