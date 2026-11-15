const authDebugStorageKey = "horizon-auth-debug";
const pageId =
  globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

export function createAuthDebugId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

export function authDebug(
  event: string,
  details: Record<string, boolean | number | string | null>,
): void {
  try {
    if (window.localStorage.getItem(authDebugStorageKey) !== "1") return;
  } catch {
    return;
  }

  console.info("[horizon-auth-debug]", {
    pageId,
    elapsedMs: Math.round(performance.now()),
    event,
    ...details,
  });
}
