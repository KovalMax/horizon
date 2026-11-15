import { useCallback, useEffect, useState } from "react";
import { AuthScreen } from "./features/auth/AuthScreen";
import { useAuth } from "./features/auth/useAuth";
import type { AuthState } from "./features/auth/useAuth";
import { PlannerShell } from "./features/planner/PlannerShell";
import { usePlannerApp } from "./features/planner/usePlannerApp";

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
  if (authState.restoring) {
    return (
      <main className="welcome-card auth-card" role="status">
        Restoring your Horizon session…
      </main>
    );
  }
  return authState.tokens ? (
    <AuthenticatedPlanner authState={authState} />
  ) : (
    <UnauthenticatedApp authState={authState} />
  );
}
