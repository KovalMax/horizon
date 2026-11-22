---
name: horizon-quality
description: Use to choose regression test layers, run Playwright and quality gates, or measure API and database performance.
---

# Horizon quality gates

| Tool | Use |
| --- | --- |
| `make test-web` | Vitest/Testing Library: rendering, role affordances, retries and workflows. |
| `make test-api` | Rust tests against the running local stack; use `make check` for isolated PostgreSQL. |
| `make check` | Isolated code checks: Rust fmt/Clippy/tests and web formatting/build/tests. Does not run Playwright. |
| `make test-browser` | Deterministic Chromium, stubbed failure/auth cases, mobile sign-in axe. |
| `make test-e2e-stack` | Real HTTPS, Mailpit, Actix and PostgreSQL; authenticated roles, persistence, keyboard and UX regressions. |
| `make perf-baseline` | Release-mode synthetic API/DB/browser measurements; report only. |

- Test public behavior at the lowest layer that can catch the failure. Use
  accessible locators in browser tests; use API tests for authorization and
  persistence. Do not substitute axe for keyboard, zoom or real-device review.
  See `apps/web/e2e/README.md` and `infra/perf/README.md`.
- The signed-in All Spaces reload budget is at most **9** API requests,
  exactly one current invitation fetch, and no deferred management reads.
  Assert request counts, not a latency threshold, in ordinary PR CI.
- Compare the same seeded fixtures and runner conditions. Separate browser
  requests, HTTP latency, PostgreSQL statement/plan cost, and application
  phases. Run `EXPLAIN ANALYZE` for writes only on disposable data. Calendar
  phase profiling is opt-in in the benchmark, contains no identifiers, and
  is not application-wide tracing. Never run load tests on production.
- Prior work reduced Calendar response preparation and moved-override cost
  without changing SQL or ordering. Candidate follow-ups: deep To-dos pages,
  unchanged reminder upserts, and Notification fan-out. Re-measure before
  optimizing; do not promote one workstation run into a release threshold.
