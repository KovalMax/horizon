import { authDebug } from "./authDebug";

export type AuthAccount = {
  id: string;
  email: string;
};

export type AuthTokens = {
  accessToken: string;
  refreshToken: string;
};

export type AuthTokenChangeReason = "updated" | "signed-out" | "expired";

type AuthTokenListener = (
  tokens: AuthTokens | null,
  reason: AuthTokenChangeReason,
) => void;

let currentTokens: AuthTokens | null = null;
let refreshInFlight: Promise<AuthTokens | null> | null = null;
const tokenListeners = new Set<AuthTokenListener>();
const refreshTokenStorageKey = "horizon-refresh-token";

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === refreshTokenStorageKey && event.newValue === null) {
      currentTokens = null;
      for (const listener of tokenListeners) listener(null, "signed-out");
    }
  });
}

function storedRefreshToken(): string | null {
  try {
    const persistentToken = window.localStorage.getItem(refreshTokenStorageKey);
    if (persistentToken) return persistentToken;
    const previousTabToken = window.sessionStorage.getItem(
      refreshTokenStorageKey,
    );
    if (previousTabToken) {
      window.localStorage.setItem(refreshTokenStorageKey, previousTabToken);
      window.sessionStorage.removeItem(refreshTokenStorageKey);
    }
    return previousTabToken;
  } catch {
    return null;
  }
}

export function hasStoredRefreshToken(): boolean {
  return Boolean(storedRefreshToken());
}

function storeRefreshToken(token: string | null): void {
  try {
    if (token) window.localStorage.setItem(refreshTokenStorageKey, token);
    else window.localStorage.removeItem(refreshTokenStorageKey);
  } catch {
    // Keep the in-memory session usable when storage is disabled or unavailable.
  }
}

async function withRefreshLock<T>(operation: () => Promise<T>): Promise<T> {
  if (navigator.locks) {
    return navigator.locks.request("horizon-auth-refresh", operation);
  }
  return operation();
}

export function setAuthTokens(
  tokens: AuthTokens | null,
  reason: AuthTokenChangeReason = "updated",
): void {
  currentTokens = tokens;
  storeRefreshToken(tokens?.refreshToken ?? null);
  authDebug("tokens-changed", {
    reason,
    hasTokens: Boolean(tokens),
  });
  for (const listener of tokenListeners) listener(tokens, reason);
}

export function getAuthTokens(): AuthTokens | null {
  return currentTokens;
}

export function subscribeAuthTokens(listener: AuthTokenListener): () => void {
  tokenListeners.add(listener);
  return () => tokenListeners.delete(listener);
}

/** Rotates the persisted refresh token after a full page reload. */
export async function restoreAuthSession(): Promise<AuthTokens | null> {
  return withRefreshLock(async () => {
    const refreshToken = storedRefreshToken();
    if (!refreshToken) return null;
    try {
      const response = await fetch("/api/auth/token/refresh", {
        method: "POST",
        headers: { Authorization: `Bearer ${refreshToken}` },
      });
      if (response.status === 401) {
        if (storedRefreshToken() === refreshToken) storeRefreshToken(null);
        return null;
      }
      if (!response.ok) return null;
      const responseTokens = (await response.json()) as {
        access_token: string;
        refresh_token: string;
      };
      const tokens = {
        accessToken: responseTokens.access_token,
        refreshToken: responseTokens.refresh_token,
      };
      setAuthTokens(tokens);
      return tokens;
    } catch {
      return null;
    }
  });
}

async function refreshTokensFor(
  accessToken: string,
): Promise<AuthTokens | null> {
  if (currentTokens?.accessToken !== accessToken) return currentTokens;
  if (refreshInFlight) return refreshInFlight;

  const refreshToken = currentTokens.refreshToken;
  const pendingRefresh = withRefreshLock(async () => {
    try {
      const latestRefreshToken = storedRefreshToken() ?? refreshToken;
      const response = await fetch("/api/auth/token/refresh", {
        method: "POST",
        headers: { Authorization: `Bearer ${latestRefreshToken}` },
      });
      if (response.status === 401) {
        if (storedRefreshToken() === latestRefreshToken) {
          setAuthTokens(null, "expired");
        }
        return null;
      }
      if (!response.ok) return null;
      const responseTokens = (await response.json()) as {
        access_token: string;
        refresh_token: string;
      };
      const updatedTokens = {
        accessToken: responseTokens.access_token,
        refreshToken: responseTokens.refresh_token,
      };
      setAuthTokens(updatedTokens);
      return updatedTokens;
    } catch {
      return null;
    }
  });
  refreshInFlight = pendingRefresh;
  try {
    return await pendingRefresh;
  } finally {
    if (refreshInFlight === pendingRefresh) refreshInFlight = null;
  }
}

async function fetchWithAccessToken(
  input: RequestInfo | URL,
  init: RequestInit,
  accessToken: string,
): Promise<Response> {
  const sourceHeaders = init.headers;
  const headers: Record<string, string> = {};
  if (
    sourceHeaders &&
    !Array.isArray(sourceHeaders) &&
    !(sourceHeaders instanceof Headers)
  ) {
    Object.assign(headers, sourceHeaders);
  } else {
    new Headers(sourceHeaders).forEach((value, key) => {
      headers[key] = value;
    });
  }
  const authorizationKey =
    Object.keys(headers).find((key) => key.toLowerCase() === "authorization") ??
    "Authorization";
  headers[authorizationKey] = `Bearer ${accessToken}`;
  return fetch(input, { ...init, headers });
}

/** Sends an authenticated request, rotating and retrying once on an expired access token. */
export async function authenticatedFetch(
  input: RequestInfo | URL,
  init: RequestInit = {},
  fallbackAccessToken?: string,
): Promise<Response> {
  const headers = new Headers(init.headers);
  const tokenFromHeader = headers
    .get("Authorization")
    ?.replace(/^Bearer\s+/i, "");
  const accessToken =
    currentTokens?.accessToken ?? fallbackAccessToken ?? tokenFromHeader;
  if (!accessToken) return fetch(input, init);

  const response = await fetchWithAccessToken(input, init, accessToken);
  if (response.status !== 401 || !currentTokens?.refreshToken) return response;

  const refreshedTokens = await refreshTokensFor(accessToken);
  if (!refreshedTokens) return response;
  return fetchWithAccessToken(input, init, refreshedTokens.accessToken);
}

type AuthenticationResponse = {
  account: AuthAccount;
  tokens: {
    access_token: string;
    refresh_token: string;
  };
};

export class AuthApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly retryAfterSeconds: number | null = null,
  ) {
    super("The request could not be completed.");
  }
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`/api${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const retryAfter = Number(response.headers.get("Retry-After"));
    throw new AuthApiError(
      response.status,
      Number.isFinite(retryAfter) && retryAfter > 0 ? retryAfter : null,
    );
  }
  return response.json() as Promise<T>;
}

export async function requestMagicLink(
  email: string,
  timeZone: string,
): Promise<void> {
  await postJson<{ message: string }>("/auth/magic-links", {
    email,
    time_zone: timeZone,
  });
}

export async function redeemMagicLink(token: string): Promise<AuthTokens> {
  const response = await postJson<AuthenticationResponse>(
    "/auth/magic-links/redeem",
    { token },
  );
  return {
    accessToken: response.tokens.access_token,
    refreshToken: response.tokens.refresh_token,
  };
}

export async function getCurrentAccount(
  accessToken: string,
): Promise<AuthAccount> {
  const response = await authenticatedFetch("/api/account/me", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) throw new AuthApiError(response.status);
  return response.json() as Promise<AuthAccount>;
}

export async function revokeCurrentSession(
  refreshToken: string,
): Promise<void> {
  const response = await fetch("/api/auth/session/current", {
    method: "DELETE",
    headers: { Authorization: `Bearer ${refreshToken}` },
  });
  if (!response.ok) throw new AuthApiError(response.status);
}
