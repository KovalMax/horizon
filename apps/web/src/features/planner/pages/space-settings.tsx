import type { PlannerAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function SpaceSettingsPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const {
    space,
    spaceMembers,
    membershipStatus,
    membershipError,
    transferOwnership,
    deleteCurrentSpace,
    mutedSpaces,
    notificationPrefsStatus,
    notificationPrefsError,
    setSpaceMuted,
  } = state;
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader(
        "Space settings",
        space.role === "Owner"
          ? `Manage ${space.name}.`
          : `Only the Owner can transfer ownership or delete ${space.name}.`,
      )}
      <section className="list-card">
        <h2>Notifications</h2>
        <label className="space-mute-row">
          <span>
            <b>Mute this Space</b>
            <small>
              No in-app or email notifications from {space.name}. You can still
              open it any time.
            </small>
          </span>
          <input
            type="checkbox"
            role="switch"
            aria-label={`Mute ${space.name} notifications`}
            checked={mutedSpaces.includes(space.id)}
            disabled={
              notificationPrefsStatus === "loading" ||
              notificationPrefsStatus === "saving" ||
              notificationPrefsStatus === "error"
            }
            onChange={(event) =>
              void setSpaceMuted(space.id, event.currentTarget.checked)
            }
          />
        </label>
        {notificationPrefsStatus === "loading" && (
          <p role="status">Loading notification preferences…</p>
        )}
        {notificationPrefsStatus === "saving" && (
          <p role="status">Saving notification preference…</p>
        )}
        {notificationPrefsError && <p role="alert">{notificationPrefsError}</p>}
      </section>
      <section className="list-card">
        <h2>Ownership & leaving</h2>
        {space.role === "Owner" && space.tone === "personal" ? (
          <p>
            Personal Spaces stay tied to their account and can’t be transferred.
            Share the Space with Members or delete it instead.
          </p>
        ) : space.role === "Owner" ? (
          <>
            <p>
              Transfer this shared Space to an existing Member. You’ll become an
              Editor and can leave or be removed afterward.
            </p>
            <form
              className="invite-compose"
              onSubmit={async (event) => {
                event.preventDefault();
                const data = new FormData(event.currentTarget);
                const newOwnerId = String(data.get("newOwnerId") ?? "");
                const newOwner = spaceMembers.find(
                  (member) => member.user_id === newOwnerId,
                );
                if (!newOwner) return;
                state.requestConfirmation({
                  title: `Transfer ownership of ${space.name}?`,
                  description: `${newOwner.email} will become the Owner. You will become an Editor.`,
                  confirmLabel: "Transfer ownership",
                  onConfirm: () => transferOwnership(newOwnerId),
                });
              }}
            >
              <label className="form-field">
                New Owner
                <select name="newOwnerId" required defaultValue="">
                  <option value="" disabled>
                    Select a Member
                  </option>
                  {spaceMembers
                    .filter((member) => member.role !== "owner")
                    .map((member) => (
                      <option key={member.user_id} value={member.user_id}>
                        {member.email} · {member.role}
                      </option>
                    ))}
                </select>
              </label>
              <Button
                type="submit"
                disabled={
                  membershipStatus === "loading" ||
                  spaceMembers.filter((member) => member.role !== "owner")
                    .length === 0
                }
              >
                Transfer ownership
              </Button>
            </form>
            {spaceMembers.length < 2 && (
              <p>Add another Member before transferring ownership.</p>
            )}
          </>
        ) : (
          <p>
            Only the Owner can transfer ownership or delete this Space. Ask the
            Owner to remove you if you need to leave.
          </p>
        )}
      </section>
      {space.role === "Owner" && (
        <section className="list-card danger-card">
          <h2>Delete this Space</h2>
          <p>
            Deleting removes access immediately. You can restore the Space with
            its existing Members and content for seven days. Pending invitations
            and all join links will be invalidated.
          </p>
          {membershipError && <p role="alert">{membershipError}</p>}
          <Button
            variant="outline"
            onClick={() => {
              state.requestConfirmation({
                title: `Delete ${space.name}?`,
                description: `Access will be removed immediately. You can restore this Space for seven days; after that, it and its content will be permanently deleted. Pending invitations and join links will be invalidated.`,
                confirmLabel: "Delete Space",
                requiredText: space.name,
                onConfirm: () => deleteCurrentSpace(),
              });
            }}
          >
            Delete Space
          </Button>
        </section>
      )}
    </>
  );
}
