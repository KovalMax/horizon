# 03 — Show service readiness in the web client

Status: implemented locally; browser verification and review pending
Blocked by: 02

Connect a minimal React/TypeScript page to the API readiness interface. Present loading, success, and failure states. Decide and document the local browser-to-API routing approach; do not expose private credentials.

## Acceptance criteria

- A browser opened on the host can trigger or observe an API request that verifies the real database-backed path.
- The page distinguishes loading, available, and unavailable states without falsely reporting success.
- Containerized automated tests cover user-visible success and failure. A documented integration check exercises the real web-to-API-to-database path.
- Public browser configuration and server-side environment variables are clearly distinguished; a Compose-only hostname is not assumed to resolve in the user's browser.
