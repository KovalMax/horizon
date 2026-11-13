# 02 — Connect frontend magic-link sign-in

Status: implemented locally; browser/Mailpit verification pending

## Goal

Connect the web sign-in screen to the existing Horizon authentication API without changing the prototype planner into a persisted product flow.

## Scope

- Request a magic link through the same-origin `/api` route and show an account-neutral confirmation.
- Redeem the one-time token from the URL fragment at `/auth/complete`, remove the fragment before exchanging it, and load the current account from `/account/me`.
- Keep access and refresh tokens in memory only; revoke the current refresh Session on sign-out.
- Keep local email delivery on Mailpit. Do not add production email delivery or persistent browser token storage.
- Keep calendar, Tasks, Saved places, and Spaces prototype data fictional and session-only.

## Completion criteria

- Frontend workflow tests cover request, successful redemption, invalid-link feedback, and sign-out/revocation.
- The web build and frontend tests pass.
- Browser verification confirms a request appears in local Mailpit and its link signs in through the local HTTPS app.
