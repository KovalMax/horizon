# Horizon agent guide

Horizon is a mobile-first planner built with React/TypeScript/Vite, Rust/Actix,
and PostgreSQL. Read [GLOSSARY.md](GLOSSARY.md) before changing product terms or
behavior. Check `docs/adr/` if present; surface conflicts with existing decisions.

## Always

- Use the root `Makefile` and Docker for Horizon. Do not run services, builds,
  tests, formatters, or migrations on the host or invoke Docker directly.
  Start with `make help`; prefer `make check` for isolated code checks.
- Use `https://horizon.localhost/` for live browser work. Never point test
  fixtures, Mailpit, or load tools at production.
- Keep changes scoped and feature-oriented. Put Rust tests in `apps/api/tests/`
  and web tests in `apps/web/tests/` or the appropriate browser suite. Test
  observable behavior and authorization.
- Keep local specs in `.local/internal/scratch/<feature>/spec.md`, with
  numbered files under `issues/` and follow-up decisions under `## Comments`.
  Write ADRs only for durable decisions. `.local/` is ignored: no secrets and
  no CI inputs from it.
- Report what passed, what was blocked, and whether performance numbers are
  local observations or production measurements.

## Load the relevant skills

Read the linked `SKILL.md` directly if the harness does not discover project
skills. Use multiple when a change crosses boundaries.

| When working on | Skill |
| --- | --- |
| Modules, responsibilities, and domain boundaries | [Architecture](.agents/skills/horizon-architecture/SKILL.md) |
| React features, UI state, and requests | [Web](.agents/skills/horizon-web/SKILL.md) |
| Actix routes, SQL, migrations, and permissions | [API](.agents/skills/horizon-api/SKILL.md) |
| Local services, certificates, and port conflicts | [Docker](.agents/skills/horizon-docker/SKILL.md) |
| Traefik, production Compose, and rollout | [Infrastructure](.agents/skills/horizon-infra/SKILL.md) |
| PRs, required checks, Actions, and releases | [GitHub](.agents/skills/horizon-github/SKILL.md) |
| Tests, browser journeys, and performance baselines | [Quality gates](.agents/skills/horizon-quality/SKILL.md) |
| SMTP, test inboxes, and magic-link delivery | [Mailpit](.agents/skills/horizon-mailpit/SKILL.md) |
| Sessions, token storage, and rotation | [Auth tokens](.agents/skills/horizon-auth/SKILL.md) |
| Design comparison and prototype scope | [Lovable reference](.agents/skills/horizon-lovable/SKILL.md) |
| Layout, component behavior, and accessibility | [UI design](.agents/skills/horizon-design/SKILL.md) |
| CSS tokens, identity colors, and typography | [Design tokens](.agents/skills/horizon-design-tokens/SKILL.md) |
