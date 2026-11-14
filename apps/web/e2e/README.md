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

The tests run the Vite app on `127.0.0.1:4173` and stub the sign-in request so
they do not require an API, database, or mail service. The Task journey checks
the reminder controls and creates a Task in the session-backed prototype.

The repository-level `opencode.json` configures the official Playwright MCP
server (`@playwright/mcp`) for Chromium. Restart/reload OpenCode after adding the
config, then use the `playwright` MCP tools to inspect the running app. Start it
separately with `cd apps/web && npm run dev` (normally at
`http://localhost:3000`). The MCP server and the Playwright test runner are
separate: MCP is for interactive browser control, while `npm run test:e2e`
executes the repeatable checks.
