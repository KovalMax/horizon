# Horizon

Horizon is a mobile-first planner for households, families, and groups. It brings shared calendars, to-do lists, and saved places together in Spaces, with invitations and role-based access.

The intended MVP is a responsive web application for a private beta. Anyone can create an account with a verified email; access to another person's Space is controlled by invitation or Owner approval. Backend and frontend magic-link sign-in, Space authorization, and several persisted collaboration/planning APIs and screens are implemented locally. Not every product requirement is complete. Internal specifications and roadmap are maintained outside the Git repository. Planner data is available only after sign-in and is loaded from the API.

The current technical foundation uses React with TypeScript, Rust/Actix, PostgreSQL, and Docker Compose.

For local sign-in, open `https://horizon.localhost/welcome` and inspect magic-link emails in Mailpit at `http://localhost:8025`. Access tokens stay in memory; the rotating refresh token is kept in tab-scoped session storage so a reload can restore the session. See `apps/web/e2e/README.md` for deterministic browser checks and the separate real local browser-to-API-to-PostgreSQL smoke test.

Internal release and migration operations documentation is maintained outside the Git repository.
