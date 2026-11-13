# Horizon foundation slice

Status: Draft for review; not yet implementation-ready.

## Outcome

A new developer can start a minimal Horizon web application, API, and PostgreSQL database using Docker Compose, and verify a real browser-to-API-to-database path. GitHub Actions runs the agreed checks when the project is pushed to the user's future GitHub repository.

This is a technical tracer bullet, not a completed product feature. The existing prototype and client comparison are disposable references, not production foundations.

## Decisions

- A single monorepo contains the React/TypeScript web client, Rust/Actix API, and deployment configuration. Use PostgreSQL for persistence.
- Use Docker and Docker Compose for local development. Developers need Docker/Compose, not locally installed Node, Rust, or PostgreSQL.
- Use GitHub Actions for CI; the user will create the GitHub repository. Do not create a remote, push, or configure repository secrets as part of these tickets.
- Keep Redis out of the foundation unless a demonstrated need is approved.
- Use example configuration and clearly documented local-only credentials. Never commit real secrets or expose backend secrets to the browser.
- The beta will run on a VM via SSH and production Compose, but production deployment and operational hardening are a later slice.

## User stories and acceptance criteria

1. **Repeatable startup:** A developer following the documented commands can build and start the web client, API, and PostgreSQL with Compose on a supported machine, without installing the application SDKs locally. Startup failures produce useful diagnostics.
2. **Real vertical path:** A user can open a minimal web page and see the result of an API request that verifies an actual database query. An API process being alive must not be misrepresented as database readiness. The page reports a clear failure when the API or database is unavailable.
3. **Database lifecycle:** The application applies a versioned initial migration through an explicit, documented workflow and preserves local database data across ordinary container restarts. The workflow must not silently delete existing data.
4. **Behavioral checks:** Automated checks cover successful and failed readiness behavior, including a real PostgreSQL integration test, and the client-visible success/error behavior. Tests must not pass solely because the database is mocked.
5. **Developer feedback:** Formatting, static analysis and relevant automated tests run through documented containerized commands. CI invokes equivalent checks, provisions required test services, and fails on errors.
6. **Configuration:** Local setup explains environment variable interpolation versus container environment and public browser configuration. Verify that a browser-reachable API address works from the host browser; a Compose-internal hostname alone is not a browser URL. No real secret is required for the tracer bullet.
7. **Documentation:** README describes prerequisites, setup, startup, migration, checks, and non-destructive shutdown. Explicitly distinguish persistent data from disposable containers and avoid recommending volume deletion as routine cleanup.

## Testing seam

Prefer behavior checks through the API and the browser-facing UI, with a real PostgreSQL service for the integration path. Substitute only external dependencies not being tested. Keep tests independent of internal module structure; the disposable React and Flutter comparison tests are not production prior art.

## Out of scope

Product accounts, magic links, Spaces, permissions, Tasks, Calendar, Places, notifications, Redis, production hosting, DNS/TLS, and real user data. The interface is a minimal proof of the stack, not the approved visual system implemented in full.

## Open decisions before implementation

- Select a small, documented set of frontend and Rust libraries and version-pinning/lockfile conventions; use maintained dependencies and review security advisories.
- Define the exact API response and migration invocation; avoid inventing a product-domain entity solely for a health check.
- Decide whether the local browser reaches the API through a same-origin development proxy or an explicit public URL; avoid placing secrets in frontend config.
- Set up GitHub Actions in the repo when available. Hosting, SSH deployment, secrets management, TLS, backup/restore, monitoring, and production migration procedures require a later beta-readiness spec.

## Ticket order

See `issues/` for vertically sequenced tickets and their blocking edges. Review and confirm this draft before implementation.
