/* Only the public offline fallback is cached. Never cache app pages or APIs. */
const cachePrefix = "horizon-offline-";
const cacheName = `${cachePrefix}v1`;
const offlinePage = "/offline.html";

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
