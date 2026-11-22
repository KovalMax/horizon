---
name: horizon-auth
description: Use for Horizon magic-link auth, access and refresh tokens, session restore, and sign-out.
---

# Horizon auth tokens

- Read `apps/web/src/features/auth/{api.ts,useAuth.ts}` and `apps/api/src/auth/`.
  Access tokens stay in memory. Refresh tokens currently rotate and persist
  in `localStorage`; a legacy `sessionStorage` value is migrated. This is the
  current design, not a recommendation for unrelated applications.
- Keep refresh coordination through the existing request wrapper and browser
  lock when supported. Preserve the retry-once behavior, concurrent refresh
  sharing, cross-tab signed-out handling, and restoring a session on reload.
  Do not treat a transient transport error exactly like an invalid session.
- Keep magic-link requests account-neutral, redeem one time, and remove the
  callback fragment from browser history. Sign-out revokes the session and
  clears stored state. Invitation acceptance still requires the intended
  verified email; reusable join links still require Owner approval.
- Never copy JWT secrets, bearer values, email callbacks, or refresh tokens
  into docs, telemetry or CI artifacts. Preserve existing JWT/session
  verification and send all privileged actions through server authorization.
- Validate with `make test-web`, `make test-browser`, and real Mailpit
  sign-in/sign-out via `make test-e2e-stack`.
