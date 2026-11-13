# React best practices

Project-adapted notes from [Vercel React Best Practices](https://github.com/vercel-labs/agent-skills/tree/main/skills/react-best-practices), maintained by Vercel Engineering. The upstream repository and skill are MIT-licensed. This file summarizes selected guidance relevant to Horizon's React 19 + TypeScript + Vite app; use upstream rule files for examples and full context.

## Before changing React code

- Keep screens declarative and move stateful workflows into feature hooks/application functions. Keep API, serialization, and error handling behind feature API/repository modules, not ad hoc `fetch` calls in page components.
- Prefer accessible, user-visible tests for workflows; test pure domain rules separately.
- Optimize against an identified render, bundle, or network cost. Do not add `memo`, `useMemo`, or `useCallback` automatically.

## State and rendering

- Derive values from props/state during render instead of synchronizing redundant state in effects.
- Use functional state updates when a new value depends on the previous value; use lazy `useState(() => ...)` for expensive initialization.
- Keep component definitions at module scope. Extract a child component when it needs independent state, reuse, or a meaningful render boundary.
- Use explicit conditional rendering for branches; avoid rendering long lists or expensive subtrees that are not visible.
- Preserve immutable updates and stable keys. Use a `Set`/`Map` when repeated membership or key lookups make the linear scan material.

## Async work and data loading

- Run independent requests concurrently with `Promise.all`; sequence only operations with real dependencies.
- Defer work until the branch that needs it, and cancel or ignore stale client requests when a screen unmounts or its key changes.
- Deduplicate client requests only when repeated requests are a real issue; add a cache/data-fetching dependency only when its behavior is needed.
- Keep loading, empty, error, and retry states explicit at the feature boundary.

## Bundle and browser performance

- Prefer direct imports over broad barrel imports where that materially reduces the shipped bundle.
- Lazy-load genuinely heavy, non-critical features when the user approaches or activates them; avoid dynamic-loading small screens by default.
- For long lists, consider windowing or CSS `content-visibility` after measuring; preserve keyboard and screen-reader behavior.
- Use passive scroll/touch listeners only when handlers never call `preventDefault`.

## Scope note

The upstream skill also covers Next.js Server Components, Server Actions, `after()`, and React server caching. Horizon uses a Rust/Actix API and a Vite client, so those Next.js-specific recommendations do not directly apply. Apply the underlying security, concurrency, and latency principles at the Rust API boundary instead.
