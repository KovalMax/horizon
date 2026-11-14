# Space ownership and deletion lifecycle

Status: Implemented.

## Outcome

Owners can transfer a shared Space without creating multiple Owners. An Owner can delete a personal or shared Space after confirmation, recover it for seven days, or let it be permanently purged. Deletion immediately removes Member access and invalidates pending Invitations and reusable join links.

## Decisions

- Ownership transfer applies to shared Spaces only. A personal Space remains tied to its account and cannot be transferred.
- The Owner transfers to an existing Member. The change is immediate after explicit confirmation; the new Owner is promoted atomically and the former Owner becomes an Editor.
- Any current Owner may delete either a personal or shared Space after explicit confirmation.
- Only the Owner who initiated deletion may restore the Space, and only before the seven-day deadline.
- Deletion preserves content and Member roles, but removes active memberships so existing membership-scoped APIs deny access. Restore reinstates those roles and content.
- Deletion revokes pending email Invitations and all reusable join links. Existing join requests remain pending but cannot be reviewed while the Space is deleted; they can be reviewed after restoration. Revoked links and Invitations are not reactivated.
- The restore window ends at `deleted_at + 7 days`; restore is rejected at and after that boundary. A periodic API background task permanently deletes expired Spaces and their cascaded content. Purge checks run hourly, including on process startup.

## HTTP contract

- `PATCH /api/spaces/{space_id}/owner` accepts `new_owner_id`; only the current Owner may transfer a shared Space.
- `DELETE /api/spaces/{space_id}` soft-deletes a Space and returns `deleted_at` and `restore_until`.
- `GET /api/spaces/deleted` lists recoverable Spaces deleted by the caller.
- `POST /api/spaces/{space_id}/restore` restores a Space for its deleting Owner before the deadline.
- Active Space listing excludes deleted Spaces. Deleted Spaces are not accessible through existing membership-scoped APIs.

## Verification

- PostgreSQL integration coverage verifies the transfer roles and personal-Space restriction; non-Owner authorization; immediate access removal; Invitation and link invalidation; pending-request preservation; restore authorization and role restoration; and denial/purge at the seven-day boundary.
- Web workflow coverage verifies confirmation, ownership transfer, deletion, and restore through Recently deleted.
