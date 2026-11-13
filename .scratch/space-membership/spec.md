# Space Members and email invitations

Status: Implemented.

## Outcome

Authenticated Members can see the real Member directory for a Space. Owners can invite an email address; the recipient can accept after signing in with that verified address and joins as a Viewer.

## Contract

- Membership reads are Space-scoped and require membership in the Space.
- Only Owners can issue email invitations; Editors and Viewers cannot manage membership.
- Invitations are email-bound, expire, and can be accepted once by an authenticated account with the matching verified email.
- Accepted invitees become Viewers. Existing members cannot be added again through an invitation.
- Role changes, invite revocation, join links, join-request approvals, and Member removal are deferred.
- Sample Members remain prototype-only when signed out.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies Member scope, invitation delivery, access control, expiry/acceptance, and resulting membership.
- React workflow verifies real Member loading, issuing an invite, and accepting an invitation after sign-in.
