export type AuthAccount = {
  id: string;
  email: string;
};

export type AuthTokens = {
  accessToken: string;
  refreshToken: string;
};

type AuthenticationResponse = {
  account: AuthAccount;
  tokens: {
    access_token: string;
    refresh_token: string;
  };
};

export class AuthApiError extends Error {
  constructor(public readonly status: number) {
    super("The request could not be completed.");
  }
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`/api${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new AuthApiError(response.status);
  return response.json() as Promise<T>;
}

export async function requestMagicLink(email: string): Promise<void> {
  await postJson<{ message: string }>("/auth/magic-links", { email });
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
  const response = await fetch("/api/account/me", {
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
