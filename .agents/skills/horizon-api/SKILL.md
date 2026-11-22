---
name: horizon-api
description: Use for the Rust Actix API, API contracts, PostgreSQL repositories, migrations, and access rules.
---

# Horizon API

- Work by capability in `apps/api/src/`. Controllers validate transport input,
  call application services and translate results; workflows belong in
  services, invariants in models, SQL and row mapping in repositories.
  Module roots are wiring and re-exports. Keep DTOs separate when transport
  shape, validation, or privacy differs.
- Bind SQL parameters. Scope both reads and writes using authenticated
  membership; verify Owner/Editor/Viewer and cross-Space behavior in
  `apps/api/tests/`. Preserve status codes and `application/problem+json`
  contracts. Update OpenAPI annotations when changing a public contract.
- Preserve migration history; add migrations for schema changes. Test fresh
  and populated upgrades when relevant, and consider compatibility with the
  previous image: release rollback does not reverse applied SQL migrations.
- Calendar range behavior includes timezone/DST semantics, single occurrence
  versus series, and moved-in/cancelled overrides. Preserve stable
  `(starts_at, event id, occurrence date)` ordering and next offsets. Convert
  only the returned page to response DTOs; avoid repeated scans for overrides.
- Saved places list accepts a Space/category filter before pagination. Scoped
  reads return full Space category counts independent of category and offset;
  preserve the existing unscoped response when no Space is passed.
- Check changed behavior with PostgreSQL using `make check` or `make test-api`
  when the dev stack is running. Measure query changes using
  [horizon-quality](../horizon-quality/SKILL.md).
