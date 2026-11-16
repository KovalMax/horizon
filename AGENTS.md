# Horizon agent guide

## Domain and product language

- Horizon is a single-context project. Read the root `GLOSSARY.md` before changing
  product terminology; use its canonical terms in code, tests, specs, and docs.
- Check `docs/adr/` for relevant decisions when it exists. Create ADRs only when
  a durable domain or architecture decision has actually been made.
- Keep product contexts separate. Surface conflicts with established decisions
  instead of silently overriding them.

## Local development environment

- Run Horizon exclusively through the local Docker setup. Do not start services,
  run builds, tests, formatters, migrations, or application commands directly on
  the host.
- The root `Makefile` is the source of truth for Docker lifecycle and
  development commands. Use its targets for starting, stopping, restarting,
  building, inspecting, logs, migrations, and checks; do not invoke `docker`
  or `docker compose` directly.
- Prefer `make check` for the full isolated build, formatting, lint, and test
  suite. Use `make test-web` or `make test-api` for focused suites, following
  the Makefile's service/database requirements.
- Use `https://horizon.localhost/` as the local app URL for browser and
  Playwright checks. Do not use `localhost:3000` or another host-served app.
- These rules apply to all agents and subagents working in this repository.

## Architecture and organization

Use domain-driven design to keep product rules explicit and SOLID as a practical
guide. Apply both proportionally: prefer the simplest structure that protects a
real boundary; avoid empty layers, one-file-per-type scaffolding, and speculative
abstractions.

- Keep each module's responsibilities cohesive. Extend behavior behind stable
  boundaries when variation is expected; preserve the documented behavior of
  implementations; keep interfaces focused on caller needs; and make business
  rules depend on small abstractions only where real external boundaries exist.
- Do not introduce interfaces solely to satisfy a pattern. Prefer concrete,
  testable code until there is a meaningful boundary.

### Rust API

- Organize by feature/domain capability, then by responsibility. Keep HTTP,
  application, domain, and persistence boundaries clear.
- Keep controllers thin: validate transport input, call an application service,
  and translate results to HTTP. Keep business workflows out of handlers.
- Keep request/response DTOs separate from domain models when their validation,
  privacy, or representation differs.
- Put SQL and row mapping in repositories. Define repository contracts at the
  application/domain boundary when a meaningful boundary exists.
- Keep invariants near their owning models. Use factories only when creation has
  meaningful rules or coordination.
- Module roots (`mod.rs` or equivalent) are wiring and re-exports only.
- Put Rust tests in separate files under `apps/api/tests/`, not in `src/`.
  Prefer integration tests of public behavior; keep fixtures in the test tree.
  Name tests after the feature or behavior and cover relevant failure and
  authorization cases.

### React web app

- Organize by feature/domain capability. Keep pages declarative; put workflows
  in feature hooks/application functions, API calls in feature API/repository
  modules, and reusable presentation in shared UI only when it has a stable
  interface.
- Keep components independent of routing and network details. Avoid monolithic
  app/page components and ad-hoc `fetch` calls in UI components.
- Keep domain/UI types and schemas at clear boundaries; separate API DTOs where
  their shape differs from the UI model.
- Use factories/mappers for non-trivial creation or transformation rules, not
  wrappers that only rename a one-line operation.
- Put web tests in separate files under `apps/web/tests/`, not alongside
  production files in `src/`. Prefer accessible, user-visible behavior and
  workflow tests; test pure domain rules separately.
- Tests currently embedded in Rust or React source are legacy exceptions. Move
  them only when changing the related area; do not broaden a scoped task into an
  unrelated test-architecture rewrite.

## React correctness and performance

- Derive values during render rather than syncing redundant state in effects.
  Use functional state updates when the next value depends on the previous one,
  and lazy state initialization for expensive setup.
- Keep component definitions at module scope. Extract children for independent
  state, reuse, or a meaningful render boundary.
- Preserve immutable updates and stable keys. Use `Set`/`Map` when repeated
  membership or key lookups make a linear scan material.
- Render branches explicitly and avoid rendering long lists or expensive
  subtrees that are not visible.
- Run independent requests concurrently; cancel or ignore stale requests when a
  screen unmounts or its key changes. Defer work until needed and keep loading,
  empty, error, and retry states explicit.
- Optimize against an identified render, bundle, or network cost. Do not add
  `memo`, `useMemo`, or `useCallback` automatically.
- Prefer direct imports where they materially reduce the shipped bundle. Lazy
  load genuinely heavy, non-critical features; consider list windowing only
  after measuring.
- Use passive scroll/touch listeners only when their handlers never call
  `preventDefault`.
- Horizon uses React + TypeScript + Vite and a Rust/Actix API. Do not apply
  Next.js-only guidance; apply the underlying security, concurrency, and latency
  principles at the relevant boundary.

## Local specs and issues

- Project specs and issues are local Markdown under
  `.local/internal/scratch/<feature>/`.
- If a general-purpose skill suggests `.scratch/`, use this repository's
  `.local/internal/scratch/` location instead.
- Keep one feature per directory; use `spec.md` for the feature spec and
  numbered files under `issues/` for implementation issues.
- Record follow-up decisions or discussion under a `## Comments` heading in the
  relevant document.
- `.local/` is Git-ignored. These local notes are not shared with collaborators;
  do not put secrets there or describe them as version-controlled.
