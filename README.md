# Horizon

Horizon is a mobile-first planner for households, families, and groups. It brings shared calendars, to-do lists, and saved places together in Spaces, with invitations and role-based access.

The intended MVP is a responsive web application for a private beta. Anyone can create an account with a verified email; access to another person's Space is controlled by invitation or Owner approval. Backend and frontend magic-link sign-in are implemented locally. Space authorization and connected collaboration/planning features remain to be built; planner screens still use fictional, session-only prototype data.

The current technical foundation uses React with TypeScript, Rust/Actix, PostgreSQL, and Docker Compose.

For local sign-in, open `https://horizon.localhost/welcome` and inspect magic-link emails in Mailpit at `http://localhost:8025`. Authentication tokens stay in memory; the planner prototype remains fictional and session-only.
