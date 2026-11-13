## Agent skills

### Architecture and code organization

Follow `docs/agents/architecture.md` for Rust and React structure, testing location, and design principles.

### Issue tracker

Specs and issues are tracked as local Markdown under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context. See `docs/agents/domain.md`.

### React performance guidance

For React changes, apply the project-adapted Vercel guidance in `docs/agents/react-best-practices.md`. Treat it as review guidance: prioritize correctness and measured bottlenecks over speculative memoization, and do not apply Next.js-only rules to this Vite app.
