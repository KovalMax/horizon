import type { PrototypeAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function SpaceSettingsPage({
  state,
  helpers,
}: {
  state: PrototypeAppState;
  helpers: PageHelpers;
}) {
  const {
    space,
    routeSection,
    goSpace,
    showToast,
    setSpaceNames,
    spaceMembers,
    membershipStatus,
    membershipError,
    transferOwnership,
    deleteCurrentSpace,
  } = state;
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader("Space settings", `Manage ${space.name}.`)}
      <div className="space-page-tabs">
        {[
          ["calendar", "Calendar"],
          ["tasks", "To-dos"],
          ["places", "Places"],
          ["members", "Members"],
          ["settings", "Space settings"],
        ].map(([key, label]) => (
          <button
            key={key}
            className={routeSection === key ? "current" : ""}
            onClick={() => goSpace(space.id, key!)}
          >
            {label}
          </button>
        ))}
      </div>
      <section className="list-card">
        <h2>Space details</h2>
        <label className="form-field">
          Name
          <input
            aria-label="Space name"
            value={space.name}
            onChange={(event) =>
              setSpaceNames((names) => ({
                ...names,
                [space.id]: event.target.value,
              }))
            }
          />
        </label>
        <Button
          onClick={() => showToast("Space name updated for this session.")}
        >
          Save changes
        </Button>
      </section>
      <section className="list-card">
        <h2>Ownership & leaving</h2>
        {space.role === "Owner" && space.tone === "personal" ? (
          <p>
            Personal Spaces stay tied to their account and can’t be transferred.
            Share the Space with Members or delete it instead.
          </p>
        ) : space.role === "Owner" && state.accessToken ? (
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
                if (
                  !window.confirm(
                    `Transfer ownership of ${space.name} to ${newOwner.email}? You will become an Editor.`,
                  )
                )
                  return;
                await transferOwnership(newOwnerId);
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
            The current Owner must transfer ownership before this Space can be
            left.
          </p>
        )}
      </section>
      {state.accessToken && space.role === "Owner" && (
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
              if (
                window.confirm(
                  `Delete ${space.name}? You can restore it for seven days. After that, it and its content will be permanently deleted.`,
                )
              ) {
                void deleteCurrentSpace();
              }
            }}
          >
            Delete Space
          </Button>
        </section>
      )}
    </>
  );
}
