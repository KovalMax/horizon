import type { PrototypeAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function SpaceSettingsPage({
  state,
  helpers,
}: {
  state: PrototypeAppState;
  helpers: PageHelpers;
}) {
  const { space, routeSection, goSpace, showToast, setSpaceNames } = state;
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
        <p>
          Owners must transfer ownership before leaving a shared Space. These
          controls are examples and do not change account access.
        </p>
        <Button
          variant="outline"
          onClick={() =>
            showToast("Ownership transfer isn't connected in this session.")
          }
        >
          Transfer ownership
        </Button>
      </section>
      <section className="list-card danger-card">
        <h2>Delete this Space</h2>
        <p>
          A deleted Space can be restored within seven days. Previous
          invitations and join links stay invalid.
        </p>
        <Button
          variant="outline"
          onClick={() =>
            showToast("Space deletion isn't connected in this session.")
          }
        >
          Delete Space
        </Button>
      </section>
    </>
  );
}
