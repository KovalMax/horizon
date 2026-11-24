/* Only the public offline fallback is cached. Never cache app pages or APIs. */
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

self.addEventListener("push", (event) => {
  let reminder;
  try {
    reminder = event.data?.json();
  } catch {
    return;
  }
  if (!reminder || !["calendar", "tasks"].includes(reminder.kind) ||
    !uuid.test(reminder.space) || !uuid.test(reminder.item) || !uuid.test(reminder.tag)) return;
  const calendar = reminder.kind === "calendar";
  const params = new URLSearchParams({ reminder: reminder.item });
  if (calendar && /^\d{4}-\d{2}-\d{2}$/.test(reminder.occurrence)) {
    params.set("occurrence", reminder.occurrence);
  }
  event.waitUntil(
    self.registration.showNotification("Horizon", {
      body: calendar ? "A Calendar event starts soon." : "You have a Task reminder.",
      icon: "/horizon-192.png",
      tag: reminder.tag,
      data: `/spaces/${reminder.space}/${reminder.kind}?${params}`,
    }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const path = event.notification.data;
  if (typeof path !== "string" || !/^\/spaces\/[a-f0-9-]+\/(?:calendar|tasks)\?reminder=/i.test(path)) return;
  event.waitUntil(self.clients.openWindow(path));
});
const cachePrefix = "horizon-offline-";
const cacheName = `${cachePrefix}v1`;
const offlinePage = "/offline.html";

self.addEventListener("message", (event) => {
  if (event.data?.type === "HORIZON_ACTIVATE_UPDATE") {
    event.waitUntil(self.skipWaiting());
  }
});

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(cacheName).then((cache) =>
      cache.add(
        new Request(offlinePage, {
          cache: "reload",
          credentials: "omit",
          redirect: "error",
        }),
      ),
    ),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(
        keys
          .filter((key) => key.startsWith(cachePrefix) && key !== cacheName)
          .map((key) => caches.delete(key)),
      );
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.mode !== "navigate" || request.method !== "GET") return;
  const url = new URL(request.url);
  if (
    url.origin !== self.location.origin ||
    /^\/(?:api|auth)(?:\/|$)/.test(url.pathname)
  ) return;

  event.respondWith(
    fetch(request).catch(async () => {
      const cache = await caches.open(cacheName);
      return (await cache.match(offlinePage)) ?? Response.error();
    }),
  );
});
