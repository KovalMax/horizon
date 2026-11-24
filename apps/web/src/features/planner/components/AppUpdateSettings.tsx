import { useState } from "react";
import { Button } from "../../../shared/ui";

function waitForState(
  worker: ServiceWorker,
  states: ReadonlyArray<ServiceWorkerState>,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      window.clearTimeout(timeout);
      worker.removeEventListener("statechange", check);
    };
    const check = () => {
      if (states.includes(worker.state)) {
        cleanup();
        resolve();
      } else if (worker.state === "redundant") {
        cleanup();
        reject(new Error("The app update could not be installed."));
      }
    };
    const timeout = window.setTimeout(() => {
      cleanup();
      reject(new Error("The app update took too long."));
    }, 10_000);
    worker.addEventListener("statechange", check);
    check();
  });
}

export async function applyAppUpdate(registration?: ServiceWorkerRegistration) {
  if (!registration) return;
  if (!registration.waiting) await registration.update();
  if (registration.installing) {
    await waitForState(registration.installing, [
      "installed",
      "activating",
      "activated",
    ]);
  }
  const waiting = registration.waiting;
  if (waiting) {
    const activated = waitForState(waiting, ["activated"]);
    waiting.postMessage({ type: "HORIZON_ACTIVATE_UPDATE" });
    await activated;
  }
}

export function AppUpdateSettings() {
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState("");

  async function update() {
    setUpdating(true);
    setError("");
    try {
      const registration = await navigator.serviceWorker.getRegistration();
      await applyAppUpdate(registration);
      // App assets can change even when the service worker file is identical.
      window.location.reload();
    } catch {
      setError("Couldn’t update Horizon. Check your connection and try again.");
      setUpdating(false);
    }
  }

  return (
    <section className="list-card" aria-label="App updates">
      <h2>App updates</h2>
      <p>Check for an update and reload Horizon. Save any changes first.</p>
      <Button variant="outline" disabled={updating} onClick={update}>
        {updating ? "Updating…" : "Update app"}
      </Button>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
