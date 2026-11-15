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
  return (
    <>
      {pageHeader("Members & sharing", `Manage access to ${space.name}.`)}
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
                  onClick={() => {
                    if (
                      window.confirm(
                        `Remove ${member.email} from ${space.name}?`,
                      )
                    ) {
                      void state.removeMember(member.user_id);
                    }
                  }}
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
                const form = event.currentTarget;
                const email = String(
                  new FormData(form).get("email") || "",
                ).trim();
                if (space.tone === "personal") {
                  const warningKey = `horizon-personal-space-share-warning:${space.id}`;
                  if (!window.localStorage.getItem(warningKey)) {
                    const confirmed = window.confirm(
                      `Sharing ${space.name} gives new Members access to its existing calendar events, to-dos, and saved places. Continue?`,
                    );
                    if (!confirmed) return;
                    window.localStorage.setItem(warningKey, "confirmed");
                  }
                }
                if (await state.inviteMember(email)) form.reset();
              }}
            >
              <input
                aria-label="Email address to invite"
                name="email"
                type="email"
                placeholder="name@example.com"
                required
              />
              <Button type="submit" disabled={state.savingInvitation}>
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
            className="invite-compose"
            onSubmit={async (event) => {
              event.preventDefault();
              const form = event.currentTarget;
              const data = new FormData(form);
              if (space.tone === "personal") {
                const warningKey = `horizon-personal-space-share-warning:${space.id}`;
                if (!window.localStorage.getItem(warningKey)) {
                  const confirmed = window.confirm(
                    `Sharing ${space.name} gives new Members access to its existing calendar events, to-dos, and saved places. Continue?`,
                  );
                  if (!confirmed) return;
                  window.localStorage.setItem(warningKey, "confirmed");
                }
              }
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
            }}
          >
            <label className="form-field">
              Role after approval
              <select name="joinLinkRole" defaultValue="viewer">
                <option value="viewer">Viewer</option>
                <option value="editor">Editor</option>
              </select>
            </label>
            <label className="form-field">
              Expires at (optional)
              <input name="joinLinkExpiresAt" type="datetime-local" />
              <small>
                Leave blank for no expiration. Your local time is used.
              </small>
            </label>
            <Button type="submit" disabled={state.savingJoinLink}>
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
