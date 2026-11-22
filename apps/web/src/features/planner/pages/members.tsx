import type { PlannerAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function MembersPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const { space, showToast } = state;
  const { pageHeader } = helpers;
  const isOwner = space.role === "Owner";
  return (
    <>
      {pageHeader(
        "Members & sharing",
        isOwner
          ? `Manage access to ${space.name}.`
          : `Only the Owner can manage membership for ${space.name}.`,
      )}
      <div className="space-title">{space.name}</div>
      {state.membershipStatus === "loading" && (
        <p role="status">Loading Space Members…</p>
      )}
      {state.membershipError && <p role="alert">{state.membershipError}</p>}
      <section className="list-card">
        <h2>Members · {state.spaceMembers.length}</h2>
        {state.spaceMembers.map((member) => (
          <div className="member-row" key={member.user_id}>
            <span className="avatar">
              {member.email.slice(0, 2).toUpperCase()}
            </span>
            <span>
              <b>{member.email}</b>
              <small>Verified account</small>
            </span>
            {space.role === "Owner" && member.role !== "owner" ? (
              <>
                <select
                  aria-label={`Role for ${member.email}`}
                  value={member.role}
                  onChange={(event) => {
                    void state.changeMemberRole(
                      member.user_id,
                      event.target.value as "editor" | "viewer",
                    );
                  }}
                >
                  <option value="editor">Editor</option>
                  <option value="viewer">Viewer</option>
                </select>
                <Button
                  variant="outline"
                  onClick={() =>
                    state.requestConfirmation({
                      title: `Remove ${member.email}?`,
                      description: `They will lose access to ${space.name}.`,
                      confirmLabel: "Remove Member",
                      onConfirm: () => state.removeMember(member.user_id),
                    })
                  }
                >
                  Remove
                </Button>
              </>
            ) : (
              <span className="role-pill">{member.role}</span>
            )}
          </div>
        ))}
        {state.membershipStatus === "ready" &&
          state.spaceMembers.length === 0 && (
            <p className="empty-inline">No Members found in this Space.</p>
          )}
      </section>
      <section className="list-card role-guidance">
        <h2>What each role can do</h2>
        <p>
          <b>Owner:</b> Manages Members and ownership, and can do everything
          Editors can.
        </p>
        <p>
          <b>Editor:</b> Can create, edit, and delete Calendar events, Tasks,
          and Saved places.
        </p>
        <p>
          <b>Viewer:</b> Can read shared content but cannot change it.
        </p>
      </section>
      {space.role === "Owner" && (
        <>
          <section className="list-card">
            <h2>Join requests · {state.pendingJoinRequests.length}</h2>
            {state.pendingJoinRequests.length === 0 ? (
              <p className="empty-inline">No pending join requests.</p>
            ) : (
              state.pendingJoinRequests.map((joinRequest) => (
                <div className="request-row" key={joinRequest.id}>
                  <div>
                    <b>
                      {joinRequest.requester_email ?? "Signed-in requester"}
                    </b>
                    <small>
                      Requested{" "}
                      {joinRequest.role === "editor" ? "Editor" : "Viewer"}{" "}
                      access ·{" "}
                      {new Date(joinRequest.requested_at).toLocaleString()}
                    </small>
                  </div>
                  <div className="request-actions">
                    <Button
                      variant="outline"
                      onClick={() =>
                        void state.decideJoinRequest(joinRequest.id, "declined")
                      }
                    >
                      Decline
                    </Button>
                    <Button
                      onClick={() =>
                        void state.decideJoinRequest(joinRequest.id, "approved")
                      }
                    >
                      Approve
                    </Button>
                  </div>
                </div>
              ))
            )}
          </section>
          <section className="list-card">
            <h2>Invite someone</h2>
            <p>
              They’ll join as a Viewer after signing in with the invited email.
            </p>
            <form
              className="invite-compose"
              onSubmit={async (event) => {
                event.preventDefault();
                if (state.membershipStatus === "loading") return;
                const form = event.currentTarget;
                const email = String(
                  new FormData(form).get("email") || "",
                ).trim();
                const sendInvitation = async () => {
                  if (await state.inviteMember(email)) form.reset();
                };
                if (space.tone === "personal") {
                  const warningKey = `horizon-personal-space-share-warning:${space.id}`;
                  if (!window.localStorage.getItem(warningKey)) {
                    state.requestConfirmation({
                      title: `Share ${space.name}?`,
                      description:
                        "New Members will get access to its existing Calendar events, To-dos, and Saved places.",
                      confirmLabel: "Continue",
                      onConfirm: async () => {
                        window.localStorage.setItem(warningKey, "confirmed");
                        await sendInvitation();
                      },
                    });
                    return;
                  }
                }
                await sendInvitation();
              }}
            >
              <input
                aria-label="Email address to invite"
                name="email"
                type="email"
                placeholder="name@example.com"
                required
              />
              <Button
                type="submit"
                disabled={
                  state.savingInvitation || state.membershipStatus === "loading"
                }
              >
                {state.savingInvitation ? "Sending…" : "＋ Invite"}
              </Button>
            </form>
            {state.spaceInvitations.map((invitation) => (
              <div className="invite-row" key={invitation.id}>
                <span>{invitation.email}</span>
                <span>Viewer · Pending</span>
                <span>
                  Expires {new Date(invitation.expires_at).toLocaleDateString()}
                </span>
                <Button
                  variant="outline"
                  onClick={() => void state.revokeInvitation(invitation.id)}
                >
                  Revoke
                </Button>
              </div>
            ))}
          </section>
        </>
      )}
      {space.role === "Owner" && (
        <section className="list-card">
          <h2>Join links</h2>
          <p>
            Anyone with a link can request the selected role. You approve each
            request before they join.
          </p>
          <form
            className="invite-compose join-link-compose"
            onSubmit={async (event) => {
              event.preventDefault();
              if (state.membershipStatus === "loading") return;
              const form = event.currentTarget;
              const data = new FormData(form);
              const createLink = async () => {
                const expiryValue = String(data.get("joinLinkExpiresAt") ?? "");
                const expiresAt = expiryValue
                  ? new Date(expiryValue).toISOString()
                  : null;
                if (
                  await state.createJoinLink(
                    String(data.get("joinLinkRole")) as "viewer" | "editor",
                    expiresAt,
                  )
                ) {
                  form.reset();
                }
              };
              if (space.tone === "personal") {
                const warningKey = `horizon-personal-space-share-warning:${space.id}`;
                if (!window.localStorage.getItem(warningKey)) {
                  state.requestConfirmation({
                    title: `Share ${space.name}?`,
                    description:
                      "Anyone using this link can request access to its existing Calendar events, To-dos, and Saved places. You must approve each request.",
                    confirmLabel: "Continue",
                    onConfirm: async () => {
                      window.localStorage.setItem(warningKey, "confirmed");
                      await createLink();
                    },
                  });
                  return;
                }
              }
              await createLink();
            }}
          >
            <label className="form-field join-link-field">
              Role after approval
              <select name="joinLinkRole" defaultValue="viewer">
                <option value="viewer">Viewer</option>
                <option value="editor">Editor</option>
              </select>
            </label>
            <label className="form-field join-link-field join-link-expiry">
              Expires at (optional)
              <input name="joinLinkExpiresAt" type="datetime-local" />
              <small>
                Leave blank for no expiration. Your local time is used.
              </small>
            </label>
            <Button
              type="submit"
              disabled={
                state.savingJoinLink || state.membershipStatus === "loading"
              }
            >
              {state.savingJoinLink ? "Creating…" : "Create join link"}
            </Button>
          </form>
          {state.spaceJoinLinks.map((link) => {
            const expired =
              link.expires_at !== null &&
              Date.parse(link.expires_at) <= Date.now();
            const inactive = Boolean(link.revoked_at) || expired;
            return (
              <div className="invite-row" key={link.id}>
                <span>
                  <b>{link.role === "editor" ? "Editor" : "Viewer"} link</b>
                  <small>
                    {link.revoked_at
                      ? "Revoked"
                      : expired
                        ? "Expired"
                        : link.expires_at
                          ? `Expires ${new Date(link.expires_at).toLocaleString()}`
                          : "Never expires"}
                  </small>
                  {!inactive && <code>{link.url}</code>}
                </span>
                {!inactive && (
                  <>
                    <Button
                      variant="outline"
                      onClick={() =>
                        void navigator.clipboard
                          ?.writeText(link.url)
                          .then(() => state.showToast("Join link copied."))
                      }
                    >
                      Copy link
                    </Button>
                    <Button
                      variant="outline"
                      onClick={() => void state.revokeJoinLink(link.id)}
                    >
                      Revoke
                    </Button>
                  </>
                )}
              </div>
            );
          })}
        </section>
      )}
    </>
  );
}
