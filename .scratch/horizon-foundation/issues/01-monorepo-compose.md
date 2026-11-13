# 01 — Establish containerized monorepo foundation

Status: draft
Blocked by: none

Create the production-oriented monorepo skeleton for React/TypeScript and Rust/Actix, plus a local Compose configuration with PostgreSQL. Use pinned or lockfile-controlled dependencies and documented local-only configuration. Developers must be able to build and start the services with Docker/Compose alone.

## Acceptance criteria

- Web, API, and database services build/start using documented commands; no local SDK installation is required.
- Local database storage survives ordinary container restarts, without any destructive cleanup in normal commands.
- No actual credentials or secrets are committed; the existing prototype and experiments remain separate.
- Document configuration, ports, startup, and non-destructive shutdown.

## Verification

Run the documented startup against a clean local configuration and confirm the browser can reach the web service. Do not discard existing volumes to perform the check.
