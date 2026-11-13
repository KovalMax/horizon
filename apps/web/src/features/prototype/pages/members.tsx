import type { PrototypeAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function MembersPage({
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
    approved,
    requestPending,
    setRequestPending,
    setApproved,
    memberRoles,
    setMemberRoles,
    invitedEmails,
    setInvitedEmails,
    showToast,
  } = state;
  const { pageHeader } = helpers;
  if (state.accessToken) {
    return (
      <>
        {pageHeader("Members & sharing", `Manage access to ${space.name}.`)}
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
              <span className="role-pill">{member.role}</span>
            </div>
          ))}
          {state.membershipStatus === "ready" &&
            state.spaceMembers.length === 0 && (
              <p className="empty-inline">No Members found in this Space.</p>
            )}
        </section>
        {space.role === "Owner" && (
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
              <Button disabled={state.savingInvitation}>
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
              </div>
            ))}
          </section>
        )}
      </>
    );
  }
  return (
    <>
      {pageHeader("Members & sharing", `Manage access to ${space.name}.`)}
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
      <div className="space-title">{space.name}</div>
      <section className="list-card">
        <h2>Members · {space.members + (approved ? 1 : 0)}</h2>
        {["Maya Okafor", "Jonas Weber", "Ruth Weber"]
          .slice(0, space.members)
          .map((name, i) => (
            <div className="member-row" key={name}>
              <span className="avatar">
                {name
                  .split(" ")
                  .map((x) => x[0])
                  .join("")}
              </span>
              <span>
                <b>{name}</b>
                <small>
                  {i === 0
                    ? "maya@example.com"
                    : `${name.split(" ")[0].toLowerCase()}@example.com`}
                </small>
              </span>
              <select
                aria-label={`Role for ${name}`}
                value={
                  memberRoles[name] ??
                  (i === 0 ? "Owner" : i === 1 ? "Editor" : "Viewer")
                }
                disabled={i === 0 || space.role !== "Owner"}
                onChange={(event) => {
                  setMemberRoles((roles) => ({
                    ...roles,
                    [name]: event.target.value,
                  }));
                  showToast(`${name}'s role changed for this session.`);
                }}
              >
                <option>Owner</option>
                <option>Editor</option>
                <option>Viewer</option>
              </select>
            </div>
          ))}
        {approved && (
          <div className="member-row">
            <span className="avatar">TR</span>
            <span>
              <b>Tomás Rivera</b>
              <small>tomas.rivera@example.com</small>
            </span>
            <span className="role-pill">Viewer</span>
          </div>
        )}
      </section>
      <section className="list-card">
        <div className="row-between">
          <div>
            <h2>Join requests</h2>
            <p>People using a link still need an Owner's approval.</p>
          </div>
        </div>
        {requestPending && !approved ? (
          <div className="request-row">
            <span className="avatar">TR</span>
            <div>
              <b>Tomás Rivera</b>
              <small>tomas.rivera@example.com · Viewer link · 45 min ago</small>
            </div>
            <div className="request-actions">
              <Button
                variant="outline"
                onClick={() => {
                  setRequestPending(false);
                  showToast("Request declined");
                }}
              >
                Decline
              </Button>
              <Button
                onClick={() => {
                  setApproved(true);
                  setRequestPending(false);
                  showToast("Tomás Rivera joined as Viewer");
                }}
              >
                Approve
              </Button>
            </div>
          </div>
        ) : (
          <p className="empty-inline">No pending requests.</p>
        )}
      </section>
      <section className="list-card">
        <div className="row-between">
          <div>
            <h2>Invite someone</h2>
            <p>Invites are simulated. No email is sent.</p>
          </div>
          <form
            className="invite-compose"
            onSubmit={(event) => {
              event.preventDefault();
              const email = String(
                new FormData(event.currentTarget).get("email") || "",
              ).trim();
              if (!email) return;
              setInvitedEmails((emails) => [...new Set([...emails, email])]);
              event.currentTarget.reset();
              showToast(
                "Invitation added for this session. No email was sent.",
              );
            }}
          >
            <input
              aria-label="Email address to invite"
              name="email"
              type="email"
              placeholder="name@example.com"
              required
            />
            <Button>＋ Invite</Button>
          </form>
        </div>
        {invitedEmails.map((email) => (
          <div className="invite-row" key={email}>
            <span>{email}</span>
            <span>Viewer · Pending</span>
            <button
              onClick={() => {
                setInvitedEmails((all) => all.filter((item) => item !== email));
                showToast("Invitation removed from this session.");
              }}
            >
              Revoke
            </button>
          </div>
        ))}
      </section>
      <section className="list-card">
        <h2>Join links</h2>
        <div className="invite-row">
          <code>LINDEN-7Q4X</code>
          <span>Viewer · Expires in 5 days</span>
          <Button variant="outline" onClick={() => showToast("Link copied")}>
            Copy
          </Button>
        </div>
        <Button
          variant="outline"
          onClick={() => showToast("New Viewer link created (simulated).")}
        >
          ＋ Create join link
        </Button>
      </section>
    </>
  );
}
