# 02 — Verify database readiness through the API

Status: implemented locally; review pending
Blocked by: 01

Add a versioned initial migration and an API readiness interface that checks a real PostgreSQL query. Keep process liveness distinct from database readiness and document how migrations run.

## Acceptance criteria

- The documented migration workflow runs against the local database and is safe to repeat according to the chosen migration tool.
- A successful readiness request reflects a successful database query; an unavailable database yields a non-success response without leaking credentials.
- Automated behavior tests exercise success and failure; at least the success path uses a real PostgreSQL instance.
- Configuration and connection failures are understandable without exposing secrets. No artificial product entity is required.
