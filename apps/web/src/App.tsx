import { PrototypeShell } from "./features/prototype/PrototypeShell";
import { usePrototypeApp } from "./features/prototype/usePrototypeApp";
import { useAuth } from "./features/auth/useAuth";

export function App() {
  const authState = useAuth();
  const appState = usePrototypeApp(authState.tokens?.accessToken ?? null);
  return <PrototypeShell appState={appState} authState={authState} />;
}
