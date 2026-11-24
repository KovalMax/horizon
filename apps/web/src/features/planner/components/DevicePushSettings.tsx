import { useEffect, useState } from "react";
import { Button } from "../../../shared/ui";
import { authenticatedFetch } from "../../auth/api";

type Preferences = { calendar_reminders: boolean; task_reminders: boolean };
const defaults: Preferences = {
  calendar_reminders: true,
  task_reminders: true,
};
const path = "/api/notifications/push-subscription";

async function subscriptionRequest(
  accessToken: string,
  method: "POST" | "PUT" | "DELETE",
  body: object,
) {
  const response = await authenticatedFetch(
    method === "POST" ? `${path}/status` : path,
    {
      method,
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    },
  );
  if (!response.ok) throw new Error("Push settings request failed.");
  return response;
}

function decodeKey(key: string) {
  const base64 = key.replaceAll("-", "+").replaceAll("_", "/");
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
}

export function DevicePushSettings({ accessToken }: { accessToken: string }) {
  const [key, setKey] = useState<string | null>(null);
  const [device, setDevice] = useState<PushSubscription | null>(null);
  const [preferences, setPreferences] = useState<Preferences>(defaults);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const supported =
    "serviceWorker" in navigator &&
    "PushManager" in window &&
    "Notification" in window;

  useEffect(() => {
    const controller = new AbortController();
    void (async () => {
      try {
        const response = await fetch("/api/notifications/push-config", {
          signal: controller.signal,
        });
        if (!response.ok) return;
        const config = (await response.json()) as { public_key: string | null };
        if (!config.public_key) return;
        if (supported) {
          const registration = await navigator.serviceWorker.getRegistration();
          const subscription =
            await registration?.pushManager.getSubscription();
          if (subscription) {
            const status = await subscriptionRequest(accessToken, "POST", {
              endpoint: subscription.endpoint,
            });
            if (status.status === 200) {
              const saved = (await status.json()) as Preferences;
              if (controller.signal.aborted) return;
              setDevice(subscription);
              setPreferences(saved);
            }
          }
        }
        if (!controller.signal.aborted) setKey(config.public_key);
      } catch {
        // Keep the experimental feature hidden if configuration cannot load.
      }
    })();
    return () => controller.abort();
  }, [accessToken, supported]);

  if (!key) return null;

  async function save(next: Preferences) {
    setBusy(true);
    setError("");
    let created: PushSubscription | null = null;
    try {
      let subscription = device;
      if (!subscription) {
        // Safari requires this call to start directly in the click handler.
        if ((await Notification.requestPermission()) !== "granted") {
          setError("Allow notifications in your browser or device settings.");
          return;
        }
        const registration = await navigator.serviceWorker.getRegistration();
        if (!registration?.active || !key) {
          setError("Reload Horizon to finish setting up this device.");
          return;
        }
        const existing = await registration.pushManager.getSubscription();
        if (existing && !(await existing.unsubscribe())) {
          throw new Error("Could not replace the browser subscription.");
        }
        subscription = created = await registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: decodeKey(key),
        });
      }
      const { keys } = subscription.toJSON();
      await subscriptionRequest(accessToken, "PUT", {
        endpoint: subscription.endpoint,
        p256dh: keys?.p256dh,
        auth: keys?.auth,
        ...next,
      });
      setDevice(subscription);
      setPreferences(next);
    } catch {
      await created?.unsubscribe().catch(() => {});
      setError("This device couldn’t save its push settings. Try again.");
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    if (!device) return;
    setBusy(true);
    setError("");
    try {
      await subscriptionRequest(accessToken, "DELETE", {
        endpoint: device.endpoint,
      });
      setDevice(null);
      await device.unsubscribe();
    } catch {
      setError("Couldn’t disconnect push on this device. Try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="list-card" aria-label="Device push setup">
      <h2>Push on this device</h2>
      <p>
        Receive Calendar and Task reminders on this device, even when Horizon is
        closed.
      </p>
      {!supported ? (
        <p>On iPhone, open Horizon from your Home Screen to set up push.</p>
      ) : device ? (
        <>
          {(
            [
              ["calendar_reminders", "Calendar reminders"],
              ["task_reminders", "Task reminders"],
            ] as const
          ).map(([kind, label]) => (
            <label className="setting-row muted-space-row" key={kind}>
              <span>{label}</span>
              <input
                type="checkbox"
                role="switch"
                checked={preferences[kind]}
                disabled={busy}
                onChange={(event) =>
                  void save({ ...preferences, [kind]: event.target.checked })
                }
              />
            </label>
          ))}
          <Button variant="outline" disabled={busy} onClick={disconnect}>
            Disconnect this device
          </Button>
        </>
      ) : (
        <Button disabled={busy} onClick={() => void save(defaults)}>
          Register this device
        </Button>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
