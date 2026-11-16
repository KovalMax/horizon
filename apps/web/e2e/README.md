# Browser tests and Playwright MCP

Install the web dependencies and Chromium once:

```sh
cd apps/web
npm ci
npx playwright install --with-deps chromium
npm run test:e2e
```

If Node.js is not installed locally, run the same checks in the matching
Playwright container from this directory:

```sh
docker run --rm -v "$PWD:/work" -w /work \
  mcr.microsoft.com/playwright:v1.65.1-noble \
  sh -lc 'npm ci && npm run test:e2e'
```

The deterministic suite runs the Vite app on `127.0.0.1:4173`. It stubs external
API responses where needed and checks sign-in requests, callback redemption and
URL cleanup, missing-token handling, retry and rate-limit feedback, and session
restoration. Authenticated persisted planner workflows are covered by web tests;
the deterministic browser suite does not replace the real local
browser-to-API-to-PostgreSQL smoke test.

## Real local browser-to-API-to-PostgreSQL smoke test

The local Compose stack bind-mounts both apps' source directories. Web edits are
picked up by Vite (polling is enabled for Docker Desktop); API edits are rebuilt
and restarted by `cargo watch`. Start or refresh the development containers with
`docker compose up --build -d`. Rust build artifacts and web dependencies live
in named volumes, not in the host source tree. If web dependencies change, restart
them with `docker compose exec web npm ci`, then run `docker compose restart web`
so the named dependency volume is refreshed.
The development API applies pending database migrations at startup and before
each watched API restart.

This separate smoke test calls the local Compose stack through the same HTTPS
host a browser uses. It has no Playwright-managed web server and does not stub
the readiness request. Run it only against the local development stack:

```sh
cd <repository-root>
./scripts/local-certs.sh
docker compose up --build -d
cd apps/web
npm run test:e2e:smoke
```

The test trusts the local certificate only for this Playwright context and
expects `https://horizon.localhost` (override with `HORIZON_SMOKE_URL`). It
asserts both the browser-visible success state and the real `/api/health/ready`
HTTP 200 response, which executes `SELECT 1` against PostgreSQL. Use `docker
compose down` for non-destructive shutdown; do not add `-v` if you want to keep
local database data. This smoke test does not cover magic-link delivery; the
separate auth/Mailpit browser journey is still an outstanding acceptance item.

The repository-level `opencode.json` configures the official Playwright MCP
server (`@playwright/mcp`) for Chromium. Restart/reload OpenCode after adding the
config, then use the `playwright` MCP tools to inspect the running app. Start it
separately with `cd apps/web && npm run dev` (normally at
`http://localhost:3000`). The MCP server and the Playwright test runner are
separate: MCP is for interactive browser control, while `npm run test:e2e`
executes the repeatable checks.
