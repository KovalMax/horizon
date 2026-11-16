import { useCallback, useEffect, useState } from "react";
import { AuthScreen } from "./features/auth/AuthScreen";
import { useAuth } from "./features/auth/useAuth";
import type { AuthState } from "./features/auth/useAuth";
import { PlannerShell } from "./features/planner/PlannerShell";
import { usePlannerApp } from "./features/planner/usePlannerApp";
import { authDebug } from "./features/auth/authDebug";
import {
  listMySpaceJoinRequests,
  requestSpaceJoin,
  type SpaceJoinRequest,
} from "./features/planner/membersApi";

function AuthenticatedJoinRequest({ authState }: { authState: AuthState }) {
  const [path, setPath] = useState(window.location.pathname);
  const [joinRequests, setJoinRequests] = useState<SpaceJoinRequest[]>([]);
  const navigate = useCallback((to: string) => {
    window.history.pushState({}, "", to);
    setPath(window.location.pathname);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, []);

  useEffect(() => {
    let active = true;
    void listMySpaceJoinRequests(authState.tokens!.accessToken)
      .then((requests) => {
        if (active) setJoinRequests(requests);
      })
      .catch(() => {
        if (active) setJoinRequests([]);
      });
    return () => {
      active = false;
    };
  }, [authState.tokens]);

  const submitRequest = async (linkId: string) => {
    try {
      const request = await requestSpaceJoin(
        authState.tokens!.accessToken,
        linkId,
      );
      setJoinRequests((current) => [
        request,
        ...current.filter((item) => item.id !== request.id),
      ]);
      return request;
    } catch {
      return null;
    }
  };

  return (
    <AuthScreen
      path={path}
      auth={authState}
      navigate={navigate}
      joinRequests={joinRequests}
      requestJoinAccess={submitRequest}
    />
  );
}

function UnauthenticatedApp({ authState }: { authState: AuthState }) {
  const [path, setPath] = useState(window.location.pathname);
  useEffect(() => {
    const updatePath = () => setPath(window.location.pathname);
    window.addEventListener("popstate", updatePath);
    return () => window.removeEventListener("popstate", updatePath);
  }, []);
  const navigate = useCallback((to: string) => {
    window.history.pushState({}, "", to);
    setPath(window.location.pathname);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, []);

  return (
    <AuthScreen
      path={path === "/auth/complete" ? path : "/welcome"}
      auth={authState}
      navigate={navigate}
    />
  );
}

function AuthenticatedPlanner({ authState }: { authState: AuthState }) {
  const appState = usePlannerApp(authState.tokens!.accessToken);
  return <PlannerShell appState={appState} authState={authState} />;
}

export function App() {
  const authState = useAuth();
  const [route, setRoute] = useState(
    () => window.location.pathname + window.location.search,
  );
  useEffect(() => {
    const updateRoute = () =>
      setRoute(window.location.pathname + window.location.search);
    window.addEventListener("popstate", updateRoute);
    return () => window.removeEventListener("popstate", updateRoute);
  }, []);
  useEffect(() => {
    authDebug("app-state", {
      mode: authState.restoring
        ? "restoring"
        : authState.tokens
          ? "authenticated"
          : "unauthenticated",
      hasTokens: Boolean(authState.tokens),
      path: window.location.pathname,
    });
  }, [authState.restoring, Boolean(authState.tokens)]);

  if (authState.restoring) {
    return (
      <main className="welcome-card auth-card" role="status">
        Restoring your Horizon session…
      </main>
    );
  }
  const [pathname, search] = route.split("?");
  const isJoinRequestRoute =
    pathname === "/welcome" &&
    Boolean(new URLSearchParams(search ?? "").get("join"));
  return authState.tokens ? (
    isJoinRequestRoute ? (
      <AuthenticatedJoinRequest authState={authState} />
    ) : (
      <AuthenticatedPlanner authState={authState} />
    )
  ) : (
    <UnauthenticatedApp authState={authState} />
  );
}
