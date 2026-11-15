// @vitest-environment jsdom
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { App } from "../src/App";
import { AuthScreen } from "../src/features/auth/AuthScreen";
import type { AuthState } from "../src/features/auth/useAuth";
import {
  authenticatedFetch,
  getAuthTokens,
  setAuthTokens,
  subscribeAuthTokens,
} from "../src/features/auth/api";

const account = { id: "user-1", email: "maya@example.com" };
const authResponse = {
  account,
  tokens: { access_token: "access-secret", refresh_token: "refresh-secret" },
};

beforeEach(() => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
});

function setPath(path: string) {
  window.history.replaceState({}, "", path);
}

function successfulAuthFetch() {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url === "/api/health/ready") return new Response("", { status: 200 });
    if (url === "/api/auth/magic-links/redeem") {
      return Response.json(authResponse);
    }
    if (url === "/api/account/me") return Response.json(account);
    if (url === "/api/auth/token/refresh")
      return Response.json({
        access_token: "restored-access",
        refresh_token: "restored-refresh",
      });
    if (url === "/api/spaces") return Response.json([]);
    if (url === "/api/join-requests/current") return Response.json([]);
    if (url === "/api/auth/session/current")
      return new Response(null, { status: 204 });
    if (url === "/api/auth/magic-links") {
      return Response.json({ message: "accepted" }, { status: 202 });
    }
    throw new Error(`Unexpected request: ${url}`);
  });
}

afterEach(() => {
  cleanup();
  setAuthTokens(null, "signed-out");
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.sessionStorage.clear();
  window.localStorage.clear();
  setPath("/");
});

test("refreshes once and retries the protected request with the rotated token pair", async () => {
  setAuthTokens({ accessToken: "expired-access", refreshToken: "refresh-old" });
  const calls: Array<{ url: string; authorization?: string }> = [];
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      const authorization =
        new Headers(init?.headers).get("Authorization") ?? undefined;
      calls.push({ url, authorization });
      if (url === "/api/auth/token/refresh")
        return Response.json({
          access_token: "access-new",
          refresh_token: "refresh-new",
        });
      if (authorization === "Bearer expired-access")
        return new Response(null, { status: 401 });
      return Response.json({ ok: true });
    },
  );
  vi.stubGlobal("fetch", fetchMock);

  const response = await authenticatedFetch("/api/protected", {
    method: "GET",
  });

  expect(response.status).toBe(200);
  expect(getAuthTokens()).toEqual({
    accessToken: "access-new",
    refreshToken: "refresh-new",
  });
  expect(calls).toEqual([
    { url: "/api/protected", authorization: "Bearer expired-access" },
    { url: "/api/auth/token/refresh", authorization: "Bearer refresh-old" },
    { url: "/api/protected", authorization: "Bearer access-new" },
  ]);
});

test("shares a refresh across concurrent unauthorized requests", async () => {
  setAuthTokens({ accessToken: "expired-access", refreshToken: "refresh-old" });
  let finishRefresh!: (response: Response) => void;
  const refreshResponse = new Promise<Response>((resolve) => {
    finishRefresh = resolve;
  });
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/auth/token/refresh") return refreshResponse;
      if (
        new Headers(init?.headers).get("Authorization") ===
        "Bearer expired-access"
      )
        return new Response(null, { status: 401 });
      return Response.json({ ok: true });
    },
  );
  vi.stubGlobal("fetch", fetchMock);

  const first = authenticatedFetch("/api/first");
  const second = authenticatedFetch("/api/second");
  await waitFor(() =>
    expect(
      fetchMock.mock.calls.filter(
        ([input]) => String(input) === "/api/auth/token/refresh",
      ),
    ).toHaveLength(1),
  );
  finishRefresh(
    Response.json({ access_token: "access-new", refresh_token: "refresh-new" }),
  );
  const [firstResponse, secondResponse] = await Promise.all([first, second]);

  expect(firstResponse.status).toBe(200);
  expect(secondResponse.status).toBe(200);
  expect(
    fetchMock.mock.calls.filter(
      ([input]) => String(input) === "/api/auth/token/refresh",
    ),
  ).toHaveLength(1);
});

test("clears the session when refresh is rejected", async () => {
  setAuthTokens({
    accessToken: "expired-access",
    refreshToken: "revoked-refresh",
  });
  const listener = vi.fn();
  const unsubscribe = subscribeAuthTokens(listener);
  const fetchMock = vi.fn(async (input: RequestInfo | URL) =>
    String(input) === "/api/auth/token/refresh"
      ? new Response(null, { status: 401 })
      : new Response(null, { status: 401 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  const response = await authenticatedFetch("/api/protected");

  expect(response.status).toBe(401);
  expect(getAuthTokens()).toBeNull();
  expect(listener).toHaveBeenCalledWith(null, "expired");
  unsubscribe();
});

test("requests a real magic link and shows an account-neutral confirmation", async () => {
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  fireEvent.change(screen.getByRole("textbox", { name: "Email address" }), {
    target: { value: "maya@example.com" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Send sign-in link" }));

  expect(
    await screen.findByText(/If that address can receive mail/),
  ).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/magic-links", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: "maya@example.com" }),
  });
});

test("redeems the URL fragment, removes it, loads the account, and keeps tokens in memory", async () => {
  setPath("/auth/complete#token=one-time-secret");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  expect(await screen.findByText("Signed in as")).toBeTruthy();
  expect(window.location.hash).toBe("");
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/magic-links/redeem", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token: "one-time-secret" }),
  });
  expect(fetchMock).toHaveBeenCalledWith("/api/account/me", {
    headers: { Authorization: "Bearer access-secret" },
  });
  expect(await screen.findByText("maya@example.com")).toBeTruthy();
});

test("leaves the callback route before submitting its one-time token", async () => {
  setPath("/auth/complete#token=one-time-secret");
  let redemptionPath: string | null = null;
  const baseFetch = successfulAuthFetch();
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    if (String(input) === "/api/auth/magic-links/redeem") {
      redemptionPath = window.location.pathname;
    }
    return baseFetch(input);
  });
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  expect(await screen.findByText("Signed in as")).toBeTruthy();
  expect(redemptionPath).toBe("/");
});

test("does not submit a redemption request when the callback has no token", async () => {
  setPath("/auth/complete");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  expect(
    await screen.findByText(
      "This sign-in link is missing its token. Request a new link.",
    ),
  ).toBeTruthy();
  expect(
    fetchMock.mock.calls.some(
      ([input]) => String(input) === "/api/auth/magic-links/redeem",
    ),
  ).toBe(false);
});

test("keeps an existing session when a callback remount has no token", async () => {
  setPath("/auth/complete");
  const auth = {
    account,
    tokens: { accessToken: "access-secret", refreshToken: "refresh-secret" },
    requestStatus: "idle",
    redemptionStatus: "idle",
    error: "",
    restoring: false,
    requestLink: vi.fn(async () => {}),
    redeemLink: vi.fn(async () => true),
    signOut: vi.fn(async () => {}),
    resetRequest: vi.fn(),
  } satisfies AuthState;
  const navigate = vi.fn();

  render(<AuthScreen path="/auth/complete" auth={auth} navigate={navigate} />);

  await waitFor(() => expect(navigate).toHaveBeenCalledWith("/"));
  expect(auth.redeemLink).not.toHaveBeenCalled();
});

test("restores the signed-in session after a page reload using the tab refresh token", async () => {
  window.sessionStorage.setItem("horizon-refresh-token", "persisted-refresh");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);

  render(<App />);

  expect(await screen.findByText("Signed in as")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/token/refresh", {
    method: "POST",
    headers: { Authorization: "Bearer persisted-refresh" },
  });
  expect(fetchMock).toHaveBeenCalledWith("/api/account/me", {
    headers: { Authorization: "Bearer restored-access" },
  });
  expect(window.sessionStorage.getItem("horizon-refresh-token")).toBe(
    "restored-refresh",
  );
  expect(getAuthTokens()).toEqual({
    accessToken: "restored-access",
    refreshToken: "restored-refresh",
  });
});

test("preserves a rotated session when the account lookup is temporarily unavailable", async () => {
  window.sessionStorage.setItem("horizon-refresh-token", "persisted-refresh");
  const successfulFetch = successfulAuthFetch();
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      if (String(input) === "/api/account/me") {
        return new Response("", { status: 503 });
      }
      return successfulFetch(input, init);
    },
  );
  vi.stubGlobal("fetch", fetchMock);

  render(<App />);

  expect(await screen.findByText(/couldn’t load your account/i)).toBeTruthy();
  expect(screen.getByRole("navigation", { name: "Spaces" })).toBeTruthy();
  expect(window.sessionStorage.getItem("horizon-refresh-token")).toBe(
    "restored-refresh",
  );
  expect(getAuthTokens()).toEqual({
    accessToken: "restored-access",
    refreshToken: "restored-refresh",
  });
});

test("explains rate limits and respects the server retry interval", async () => {
  setPath("/welcome");
  const fetchMock = vi.fn(
    async () =>
      new Response(
        JSON.stringify({
          title: "Too many requests",
          status: 429,
          code: "rate_limited",
        }),
        {
          status: 429,
          headers: {
            "Content-Type": "application/problem+json",
            "Retry-After": "60",
          },
        },
      ),
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  fireEvent.change(screen.getByRole("textbox", { name: "Email address" }), {
    target: { value: "maya@example.com" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Send sign-in link" }));

  expect(
    await screen.findByText(/wait about a minute before trying again/i),
  ).toBeTruthy();
  expect(screen.getByText(/does not affect your account/i)).toBeTruthy();
});

test("returns a signed-in requester to a join link and submits an approval request", async () => {
  const linkId = "a3ce1d3e-dcf9-4c3a-b5b8-1f607ff7e251";
  const spaceId = "b235a841-56d1-4ca9-bbea-d59821c17825";
  window.localStorage.setItem("horizon-join-link", linkId);
  setPath("/auth/complete#token=one-time-secret");
  const request = {
    id: "e474de2a-7dd2-4ef8-ae57-044dc66d3cc2",
    link_id: linkId,
    space_id: spaceId,
    space_name: "Shared household",
    requester_id: account.id,
    requester_email: null,
    role: "viewer",
    status: "pending",
    requested_at: "2026-11-13T12:00:00Z",
    decided_at: null,
  };
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/health/ready") return new Response("", { status: 200 });
      if (url === "/api/auth/magic-links/redeem")
        return Response.json(authResponse);
      if (url === "/api/account/me") return Response.json(account);
      if (url === "/api/spaces")
        return Response.json([
          {
            id: spaceId,
            name: "Shared household",
            kind: "shared",
            role: "viewer",
            member_count: 2,
          },
        ]);
      if (url.startsWith("/api/events?"))
        return Response.json({ items: [], next_offset: null });
      if (url.startsWith("/api/tasks?"))
        return Response.json({
          lists: [],
          tasks: [],
          lists_next_offset: null,
          tasks_next_offset: null,
        });
      if (url.startsWith("/api/places?"))
        return Response.json({ items: [], next_offset: null });
      if (url === `/api/spaces/${spaceId}/members`) return Response.json([]);
      if (
        url === `/api/spaces/${spaceId}/invitations` ||
        url === "/api/invitations/current"
      )
        return Response.json([]);
      if (url === "/api/join-requests/current") return Response.json([]);
      if (
        url === `/api/join-links/${linkId}/requests` &&
        init?.method === "POST"
      )
        return Response.json(request, { status: 201 });
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  fireEvent.click(
    await screen.findByRole("button", { name: "Request access" }),
  );

  expect(
    await screen.findByText("Your request is pending Owner approval."),
  ).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/join-links/${linkId}/requests`,
    expect.objectContaining({ method: "POST" }),
  );
});

test("shows an invalid-link message when redemption fails", async () => {
  setPath("/auth/complete#token=expired-secret");
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      if (String(input) === "/api/health/ready") {
        return new Response("", { status: 200 });
      }
      return new Response("{}", { status: 401 });
    }),
  );
  render(<App />);

  expect((await screen.findByRole("alert")).textContent).toContain(
    "invalid, expired, or already used",
  );
  expect(
    screen.getByRole("button", { name: "Request a new link" }),
  ).toBeTruthy();
});

test("signs out by revoking the refresh session and clears the account", async () => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  setPath("/auth/complete#token=one-time-secret");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);
  await screen.findByText("Signed in as");

  fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

  await waitFor(() =>
    expect(
      screen.getByRole("heading", { name: "Your life, across Spaces." }),
    ).toBeTruthy(),
  );
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/session/current", {
    method: "DELETE",
    headers: { Authorization: "Bearer refresh-secret" },
  });
  expect(
    await screen.findByRole("textbox", { name: "Email address" }),
  ).toBeTruthy();
});

test("signs out using the rotated refresh token after an automatic refresh", async () => {
  setPath("/auth/complete#token=one-time-secret");
  const baseFetch = successfulAuthFetch();
  let spacesRequests = 0;
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/spaces" && spacesRequests++ === 0)
        return new Response(null, { status: 401 });
      if (url === "/api/auth/token/refresh")
        return Response.json({
          access_token: "access-rotated",
          refresh_token: "refresh-rotated",
        });
      return baseFetch(input, init);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);
  await screen.findByText("Signed in as");
  await waitFor(() =>
    expect(getAuthTokens()).toEqual({
      accessToken: "access-rotated",
      refreshToken: "refresh-rotated",
    }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

  await waitFor(() =>
    expect(fetchMock).toHaveBeenCalledWith("/api/auth/session/current", {
      method: "DELETE",
      headers: { Authorization: "Bearer refresh-rotated" },
    }),
  );
});
