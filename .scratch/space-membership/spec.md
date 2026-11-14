# Space Members and email invitations

Status: Implemented; membership administration and join-request approvals added.

## Outcome

Authenticated Members can see the real Member directory for a Space. Owners can invite an email address; the recipient can accept after signing in with that verified address and joins as a Viewer. Owners can update non-Owner Members between Viewer and Editor, remove non-Owner Members, and revoke pending invitations. Owners can also create reusable join links and approve or decline requests from signed-in accounts.

## Contract

- Membership reads are Space-scoped and require membership in the Space.
- Only Owners can issue email invitations; Editors and Viewers cannot manage membership.
- Invitations are email-bound, expire, and can be accepted once by an authenticated account with the matching verified email.
- Accepted invitees become Viewers. Existing members cannot be added again through an invitation.
- Only Owners can change a non-Owner Member between Viewer and Editor, remove non-Owner Members, and revoke a pending invitation. Editors and Viewers cannot perform these operations.
- Owner roles cannot be changed and Owners cannot be removed through this workflow. Ownership transfer is a separate lifecycle feature.
- A pending invitation can be revoked; revoked, expired, and accepted invitations are terminal. A revoked or expired address can be invited again; a pending duplicate conflicts.
- In a personal Space, the browser warns the Owner before the first invitation that the invitee will be able to see existing Space content. Dismissing the warning sends no invitation; a confirmed warning is remembered for that Space in that browser.
- Owners can create multiple reusable links, choose Viewer or Editor access (never Owner), choose an expiry timestamp or no expiry, and revoke a link. Revocation blocks new requests but leaves existing requests reviewable.
- Anyone with a valid link must sign in before requesting access. Approval adds the requester with the role selected on that link; decline adds no membership. Owners see the requester's verified email; requesters see their own pending/approved/declined history.
- Only one request may be pending per account and Space. Repeated submission while pending returns the existing request; after a decision, a declined requester may use any still-valid link to request again.
- Join-request status is shown in the app; notifications are deferred to the durable notification slice.
- Sample Members remain prototype-only when signed out.

## Test seams

- Authenticated HTTP against real PostgreSQL verifies Member scope, invitation delivery, access control, role changes, removal, Owner protection, revocation/re-invitation, expiry/acceptance, and resulting membership.
- Authenticated HTTP against real PostgreSQL verifies reusable link expiry/revocation, request uniqueness/retry, Owner approval/decline, assigned roles, and requester/Owner visibility.
- React workflow verifies real Member loading, issuing an invite, and accepting an invitation after sign-in.
