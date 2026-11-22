<div align="center">
  <img src="apps/web/public/favicon.svg" width="64" height="64" alt="">
  <h1>Horizon</h1>
  <p><strong>Shared plans, without the group chat archaeology.</strong></p>
  <p>One place for the calendar, to-dos, and favorite spots you share.</p>
  <p><a href="https://horizon.false.tel/welcome"><strong>Try Horizon →</strong></a></p>
</div>

## A Space for whatever you're planning

Plan a week at home, a weekend away, or a project with friends. Give each plan
its own **Space**, invite the people involved, and keep the details where
everyone can find them.

- **Know what's coming up.** Add one-time or repeating Calendar events, and
  change a single occurrence without changing the whole series.
- **Share the work.** Make To-do lists, assign Tasks, and set due dates and
  reminders so everyone knows who's doing what.
- **Find that place again.** Save the café, grocery store, or meetup spot with
  an address and a note, right in the Space where you need it.
- **See the week in one view.** All Spaces brings your upcoming Calendar events
  and Tasks together without mixing up which Space they belong to.

## Get started

[Open Horizon](https://horizon.false.tel/welcome) and sign in with a link sent to
your email. No password to remember. Start with your Personal Space or create a
shared one. As its Owner, you decide who joins and whether they can edit or
just follow along.

Horizon is in an early beta and designed for both phones and desktops.

---

### Run Horizon locally

Horizon uses React, TypeScript, Vite, Rust/Actix, and PostgreSQL. With Docker and
Make installed, run `make help` to see the commands and `make up` to start the
local stack. Open `https://horizon.localhost/` and use Mailpit at
`http://localhost:8025` for sign-in emails. For first-time HTTPS setup, see the
[Docker guide](.agents/skills/horizon-docker/SKILL.md).

[Project conventions](AGENTS.md) · [Browser checks](apps/web/e2e/README.md) ·
[Performance baseline](infra/perf/README.md) · [Infrastructure](infra/README.md)
