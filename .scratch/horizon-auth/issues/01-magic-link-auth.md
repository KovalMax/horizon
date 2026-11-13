# 01 — MVP magic-link authentication

Status: Complete. Production email delivery remains a deployment decision outside this issue.

## Goal

Implement the first Horizon-owned authentication flow: any person can verify an email address with a one-time link, receive a personal Space on first sign-in, and use the agreed access/refresh token lifecycle. Keep authentication in the existing API process and generate the OpenAPI reference from Rust code.

## Decisions

- Account creation is open to any verified email; Space membership remains invitation- or Owner-approval-based.
- Magic-link challenge lifetime: 30 minutes; single use; persist only a hash of the secret.
- Access JWT lifetime: 10 minutes.
- Refresh credential: opaque bearer token, rotated on use, 30-day inactivity lifetime, three-month absolute Session lifetime; persist only its hash and allow revocation.
- Browser sends both credentials using `Authorization: Bearer`; keep them in memory in the client.
- PostgreSQL is authoritative; no Redis in this slice.
- Mailpit is the local-only SMTP inbox. Production email delivery remains undecided.
- Initial abuse limits: five requests per normalized email per hour, twenty per IP per hour, and a 60-second per-email resend cooldown; review before enabling external email.
- Each successful sign-in creates a separate Session; the current Session can be revoked, while session listing/revoke-all is deferred.

## Scope

- Add minimal persistence for Users, email identities, login challenges, Sessions, refresh-token rotation history, Spaces, and Space membership needed to create one personal Space.
- Add API request/response types and handlers for:
  - `POST /auth/magic-links`
  - `POST /auth/magic-links/redeem`
  - `POST /auth/token/refresh`
  - `DELETE /auth/session/current`
  - `GET /account/me`
- Annotate handlers/types so Utoipa generates the Swagger/OpenAPI contract; do not add a hand-maintained OpenAPI file.
- Send local development email through Mailpit; keep the mail sender behind a small boundary.
- Add tests for generic request responses, challenge expiry/single-use/replay, first-account personal-Space creation, token validation, refresh rotation/reuse, revocation, and account lookup.

## Security and behavior

- Magic-link request responses do not reveal whether the email already has an account.
- Consume challenges and rotate refresh credentials atomically; concurrent redemption/refresh must not issue duplicate active credentials.
- Use exact issuer/audience/expiry/signature validation for access tokens. Do not place Space roles in tokens.
- Never log raw magic-link or refresh secrets.
- Sign-out revokes the current refresh Session; access tokens already issued remain usable for at most their remaining 10-minute lifetime.
- Add bounded request throttling before sending mail. The initial values are implemented as noted above and need operational review before enabling external email.

## Out of scope

- Apple/Google sign-in, passkeys, MFA, Space invitation flows, general Space CRUD, immediate access-token revocation, production email-provider integration, and frontend sign-in screens.

## Completion criteria

- A new verified email can create an account and exactly one personal Space; an existing email signs into the same User.
- Invalid, expired, consumed, and concurrently replayed magic links cannot issue credentials.
- Access and refresh tokens are distinct; refresh tokens rotate and can be revoked; expired/invalid credentials are rejected.
- Current User endpoint is authenticated and returns only the account's public identity fields.
- Swagger UI and generated OpenAPI document include the implemented auth routes and accurate request, response, and error shapes.
- The full repository check passes, and the development API works with Mailpit without sending real email.
