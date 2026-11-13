# Backend authentication and authorization

Status: Backend MVP magic-link authentication is implemented and smoke-tested. Frontend request, redemption, account lookup, and sign-out are implemented locally; browser/Mailpit integration verification remains. Space authorization enforcement and production email delivery are not yet implemented.

## Goal

Define a Horizon-owned authentication module for the private-beta web MVP that can grow to support passkeys, Apple and Google sign-in, and multi-factor authentication (MFA). Any person may create an account with a verified email; Space membership remains separately controlled. Authentication establishes which User is acting; authorization decides what that User may do in a Space.

Horizon owns its User records, identity links, sessions, policies, and authorization decisions. Apple and Google are external identity providers, not the application's source of authorization. Biometric-capable sign-in means passkeys/WebAuthn: biometric data stays on the authenticator and is never sent to Horizon.

The API reference is generated from Rust handler and schema annotations with Utoipa; Swagger UI is enabled in development builds. Do not maintain a parallel hand-written OpenAPI document. Planned authentication endpoints appear in the generated reference when their handlers and request/response types are implemented and annotated.

## Product scope

### MVP

- Sign in or create an account with a one-time email magic link, as specified in the MVP requirements.
- Require a verified email identity for account creation and sign-in; no beta invitation is required.
- Create the user's initial personal Space after successful account creation.
- Support account sign-out and revocation of refresh sessions.
- Enforce current Space membership and role permissions on every protected operation.
- Keep Spaces private: a Space invitation or Owner-approved join request controls access to another person's Space, not access to account creation.

### Later, without changing the User model

- Add Apple and Google OpenID Connect (OIDC) identities linked to an existing Horizon User.
- Add passkeys through WebAuthn for passwordless sign-in and step-up verification.
- Add MFA policy and additional factors. Do not describe a passkey's local biometric check as a separate server-side biometric factor.

## Domain model

- **User**: Horizon account with a stable internal identifier and account lifecycle state.
- **Identity**: A verified sign-in method belonging to a User. MVP identity is an email address; later identities include an OIDC issuer/subject pair or a WebAuthn credential.
- **Authentication challenge**: Short-lived, single-use proof request, such as an email magic link or WebAuthn challenge.
- **Session**: A revocable server-side login session associated with a User and refresh-token family.
- **Space membership**: The relationship between a User and a Space, including the current role and status.

Email addresses and provider claims identify sign-in methods; application data and Space membership reference the internal User identifier. Provider email equality alone must not silently merge two accounts. A user must be authenticated to link a new provider identity, with verification of that provider identity.

## Authentication design

### Email magic link (MVP)

1. The client requests a sign-in link for an email address. Responses do not reveal whether an account already exists.
2. Apply abuse controls before sending mail: the current implementation permits at most five requests per normalized address and twenty per IP per hour, with a 60-second per-address cooldown. These values need operational review before external email is enabled.
3. Generate a cryptographically random, high-entropy challenge valid for 30 minutes. Store only a hash of its secret, along with its purpose, target email, expiry, and consumed state.
4. Send the secret in a single-use link. Redeeming it atomically consumes the challenge and verifies control of the email. Expired, consumed, or wrong-purpose challenges fail without issuing credentials.
5. Create or locate the User and email Identity, then create the first personal Space only once for a new User. Space invitations are handled separately and do not gate account creation.
6. Issue an access token and create a refresh Session. Do not put email addresses, Space roles, or other mutable authorization data in long-lived credentials.

Links must avoid leaking secrets through referrer headers, analytics, or logs. The secret is placed in the URL fragment, which is not sent in the HTTP request; a future frontend redemption flow must exchange it for a session and remove it from the browser URL. The API uses a generic accepted response and does not log link or refresh secrets.

### Email delivery

Use Mailpit as a local-development-only SMTP inbox so developers can inspect and redeem magic-link emails without sending real mail. Keep email sending behind a small adapter; choose the production email provider and sender domain separately before deployment. Do not expose Mailpit outside the local development environment.

### Access and refresh credentials (chosen direction)

- Access credential: a signed, short-lived JWT validated without a database lookup for token authenticity. Its protected header contains the configured signing algorithm, key ID (`kid`), and access-token type. Its claims contain issuer (`iss`), audience (`aud`), subject (`sub`, the internal User ID), issued-at (`iat`), and expiry (`exp`); include a random Session ID (`sid`) to associate the access token with its revocable refresh Session and support possible future session-level revocation.
- Initial access-token lifetime target: 10 minutes. An already-issued access token remains valid until expiry after its refresh Session is revoked; this bounded window is acceptable for the MVP unless a stronger revocation requirement emerges.
- Do not include Space roles or rely on role claims for authorization. Membership and role changes must take effect on the next protected request.
- Refresh credential: a distinct opaque, high-entropy bearer token with a 30-day inactivity lifetime and a three-month absolute Session lifetime. Store only its hash, associate it with a revocable server-side Session/family, and rotate atomically on use; a successful rotation renews the 30-day inactivity window but does not extend the absolute lifetime. Reuse of an already-rotated token revokes the affected family and requires reauthentication.
- Users must be able to revoke a refresh Session (including on sign-out and, later, for a lost device); account-level revocation can invalidate all Sessions. Revoking a Session prevents further refreshes. Access tokens already issued for it remain usable until their short expiry unless an online revocation check is added; this is the deliberate tradeoff for stateless access-token validation.
- Keep signing keys outside source control, support key IDs and rotation, and validate algorithm, issuer, audience, and expiry explicitly.

#### Request-token verification

1. Accept access tokens only from the `Authorization: Bearer` header over TLS; do not accept them in query parameters or silently fall back to cookies.
2. Parse with strict size/format limits. Select a verification key only from the server's configured trusted key set using `kid`; allow only the configured signing algorithm. Reject `none`, algorithm confusion, unknown keys, and token-supplied key URLs.
3. Verify the signature, exact expected issuer, expected API audience, expiry, and issued-at time (with only small documented clock skew). Require a valid subject and the access-token type; reject refresh credentials at API resource endpoints.
4. Resolve `sub` as the authenticated User. The token proves identity and validity only; before a Space operation, authorize against current membership and role in PostgreSQL.
5. Return an authentication failure for missing/invalid/expired credentials and a separate authorization failure when a valid User lacks permission. Never log bearer-token values.

Use a fixed audience for Horizon's API. Do not add Space roles or dynamic `scope` claims to the MVP token: roles are mutable and Space-specific, so they belong in the authorization check. Coarse OAuth scopes may be introduced if Horizon later supports delegated third-party API clients; they should not replace Space membership checks.

“Stateless” therefore applies to normal access-token cryptographic validation, not to refresh-session lifecycle or authorization. PostgreSQL remains the source of truth for Users, identities, sessions, invitations, and membership.

### Persistence and caching direction

- Keep PostgreSQL as the durable source of truth for Users, identities, refresh Sessions, invitations, Space membership, and authorization decisions.
- Do not add Redis or another cache to the MVP auth request path. In particular, read current membership and role from PostgreSQL so a removal or role change takes effect on the next request.
- A future cache should be selective and cache-aside: cache only measured, read-heavy data with an explicit TTL/invalidation policy; commit writes to PostgreSQL first. Cache failure must not silently grant access or lose writes.
- A cache is not a backup or high-availability plan. Treat database availability, connection limits, migrations, monitoring, and tested backup/restore as separate reliability concerns. Do not use a lagging read replica for authorization decisions where stale membership could preserve revoked access.

### Immediate access-token revocation (deferred)

If access tokens must be revocable before expiry, add a Redis-backed check keyed by a session ID or token ID, with entries expiring no later than the access token. Keep refresh Sessions and their revocation state durable in PostgreSQL; Redis must not become the only record of refresh credentials or account state.

This can be cheaper than a PostgreSQL lookup on every request, but it is slower and more operationally involved than stateless JWT validation. It also makes Redis part of the request authorization path. Define fail-closed behavior and recovery before relying on it: losing a denylist must not silently make revoked tokens valid again. Options include treating Redis loss as an authentication outage until state is rebuilt, or accepting that revocation is only guaranteed for the short access-token lifetime. Do not add Redis solely on an assumed performance benefit; benchmark and choose based on the required revocation guarantee and request load.

### Browser credential transport (current preference)

Use the `Authorization: Bearer` header for both API access and refresh requests. These are two **different credentials**, not one shared token: the access token is short-lived and signed; the refresh token is opaque, longer-lived, single-use, and rotated on use. The refresh endpoint accepts the refresh token as its bearer credential and returns a new access token and refresh token pair.

For the web client, keep both credentials in memory rather than persistent JavaScript storage. This avoids ambient-cookie authentication and suits future native clients, but a successful XSS can act as the user while the app is open, and a page reload loses both tokens and requires the user to authenticate again. Do not put a long-lived refresh secret in `localStorage` or another persistent store accessible to JavaScript. If surviving reloads becomes a requirement, revisit the storage design explicitly rather than silently persisting the refresh token. The current same-origin `/api` route supports bearer headers and does not require cross-origin cookies.

## Future authentication methods

### Apple and Google

- Implement OIDC authorization-code flow with PKCE, state, and nonce validation; validate issuer, audience, signature, expiry, and redirect URI.
- Identify an external identity by the exact `(issuer, subject)` pair, not by email alone. Treat provider email as verified only when the provider's validated claim says so.
- A signed-in User can explicitly link an OIDC identity after reauthentication. Do not auto-merge accounts based only on matching email addresses.
- Keep provider tokens out of application access tokens. Store provider tokens only if a separately specified product capability needs them; sign-in alone does not require retaining them.

### Passkeys and MFA

- Store WebAuthn credential IDs, public keys, relying-party metadata, and authenticator counters/backup properties as appropriate; never store biometric templates or private keys.
- Validate WebAuthn origin, relying-party ID, challenge, user-presence/user-verification requirements, and signature. Use discoverable credentials only with an explicit account-discovery/privacy policy.
- Treat passkeys as an authentication method. A passkey user-verification ceremony may use biometrics or a device PIN, but Horizon receives only the cryptographic assertion and verification result.
- Design factor enrollment/removal, reauthentication, recovery, and lost-device handling together. Recovery must not silently bypass MFA. Decide later whether MFA is optional or required for selected actions/accounts, and which factors/recovery codes are supported.

## Authorization design

Authorization is evaluated by the backend from the authenticated User and current resource state. Every Space-owned record is accessed through its Space boundary; a client-supplied Space ID is never proof of access. Reads and writes must be scoped in database queries or checked in the same transaction as the operation to avoid cross-Space access and time-of-check/time-of-use gaps.

Do not add global end-user roles such as `viewer`, `writer`, or `admin` for the MVP. Horizon's roles are contextual to a Space: the same User may be Owner in one Space, Editor in another, and Viewer in a third. Account eligibility or suspension is account state, not a global role. If a future operational need requires staff capabilities, define those separately from Space membership, minimize their privileges, and audit their use; do not let a platform role implicitly grant ordinary Space access.

| Action | Owner | Editor | Viewer |
| --- | --- | --- | --- |
| Read Space content | Yes | Yes | Yes |
| Create, edit, or delete shared content | Yes | Yes | No |
| Manage membership, invitations, or roles | Yes | No | No |
| Transfer ownership or delete the Space | Yes | No | No |

The Owner is the sole membership administrator, consistent with the MVP. A User may have different roles in different Spaces. Role changes and membership removal are effective from current database state on the next request; do not cache roles in tokens. Return a consistent forbidden/not-found response policy that does not disclose inaccessible Space existence.

## Data and security invariants

- A User has a stable internal ID; login identities can be added or removed without changing ownership of application data.
- A login challenge is purpose-bound, expires, and is consumed at most once, including under concurrent redemption.
- Refresh rotation and reuse detection are atomic; secrets are stored as hashes and never logged.
- Account, identity, session, invitation, and membership changes are auditable without recording credential secrets.
- Authentication endpoints have abuse controls; errors do not enable account enumeration.
- Every protected query enforces Space membership and the required role server-side.
- Email change, identity unlink, and recovery flows require verification and must not leave an account with no usable sign-in method unless that state is deliberately supported.

## Delivery sequence

1. Resolve the MVP credential transport and account/invitation policy — complete.
2. Specify schema and migration for Users, email identities, login challenges, refresh sessions, and personal-Space membership — complete for this slice.
3. Implement and test magic-link request/redemption, session issue/refresh/revoke, and initial personal-Space creation — complete.
4. Add a reusable authenticated-User extractor and Space authorization module; apply it to each protected endpoint and test role and cross-Space isolation — not started.
5. Add operational controls. Mailpit, request limits, SMTP/JWT configuration, and smoke testing exist; audit events, production delivery provider, and user-facing session management remain outstanding.
6. Add OIDC, passkeys, and MFA as separate feature specs after MVP auth is in use; do not pre-build provider-specific flows into the MVP.

## Deployment boundary

Start with authentication as a separately owned module inside the existing API application, not as a separately deployed auth service. Give the module a narrow interface for challenge redemption, session lifecycle, current-User resolution, and identity management; keep token and identity implementation details behind it. This keeps authentication replaceable and testable without adding a network dependency to onboarding, User creation, or personal-Space creation.

Consider extracting a separately deployed auth service only when there is a concrete need—such as independent scaling, multiple relying applications, separate operational ownership, or a compliance boundary. Extraction would require explicit service-to-service trust, key distribution and rotation, availability/failure behavior, and a stable protocol; it is not required to obtain a clean code boundary now.

## Open decisions before implementation

The following decisions are resolved for the implemented MVP slice:

- Browser credentials remain in memory; a reload requires sign-in.
- Immediate access-token revocation is not required; revoked refresh Sessions cannot be renewed, while issued access tokens expire within ten minutes.
- Magic links expire after 30 minutes; access tokens after 10 minutes; refresh Sessions use a 30-day inactivity and three-month absolute lifetime. Initial abuse limits are five requests per email per hour, twenty per IP per hour, and a 60-second per-email cooldown.
- Each successful sign-in creates a separate Session. The API supports revoking the current Session; list/revoke-all controls are deferred.

Remaining decisions before broader launch:

1. Which production email provider and sender/domain will be used for the private beta?
2. What account deletion and email-change behavior is required before launch?
3. For future MFA: which factors and recovery methods are acceptable, and which actions require step-up authentication?

## Acceptance criteria for the MVP auth slice

- A user can request and redeem a valid 30-minute magic link; invalid, expired, consumed, and concurrently replayed links cannot create additional sessions.
- Request responses do not reveal account existence; abuse controls prevent unbounded mail sends.
- A newly created User with a verified email gets exactly one personal Space; repeat sign-in does not create duplicates.
- Access-token signature and standard claims are validated; refresh tokens rotate and can be revoked without storing raw secrets.
- Sign-out revokes the refresh session, with the documented short access-token expiry tradeoff.
- Owner, Editor, and Viewer permissions match the matrix, and tests prove that changing IDs cannot access another Space's data.
- Current membership/role changes apply without waiting for an access token to expire.
