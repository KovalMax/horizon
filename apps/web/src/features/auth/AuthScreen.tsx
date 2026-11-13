import { useEffect, useRef, type FormEvent } from "react";
import type { AuthState } from "./useAuth";
import { Button, HorizonMark } from "../../shared/ui";

export function AuthScreen({
  path,
  auth,
  navigate,
}: {
  path: string;
  auth: AuthState;
  navigate: (path: string) => void;
}) {
  const handledCallback = useRef(false);

  useEffect(() => {
    if (path !== "/auth/complete" || handledCallback.current) return;
    handledCallback.current = true;
    const token = new URLSearchParams(window.location.hash.slice(1)).get(
      "token",
    );
    window.history.replaceState(
      window.history.state,
      "",
      `${window.location.pathname}${window.location.search}`,
    );
    void auth.redeemLink(token ?? "").then((succeeded) => {
      if (succeeded) navigate("/");
    });
  }, [auth, navigate, path]);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const email = new FormData(event.currentTarget).get("email");
    if (typeof email === "string") void auth.requestLink(email.trim());
  };

  if (path === "/auth/complete") {
    return (
      <main className="welcome-card auth-card" aria-live="polite">
        <HorizonMark large />
        <span className="eyebrow">SECURE SIGN-IN</span>
        <h1>
          {auth.redemptionStatus === "failed"
            ? "Link not accepted"
            : "Signing you in…"}
        </h1>
        {auth.redemptionStatus === "redeeming" && (
          <p role="status">Verifying your sign-in link.</p>
        )}
        {auth.error && <p role="alert">{auth.error}</p>}
        {auth.redemptionStatus === "failed" && (
          <Button onClick={() => navigate("/welcome")}>
            Request a new link
          </Button>
        )}
      </main>
    );
  }

  return (
    <main className="welcome-card auth-card">
      <HorizonMark large />
      <span className="eyebrow">WELCOME TO HORIZON</span>
      <h1>Your life, across Spaces.</h1>
      {auth.account ? (
        <>
          <p>Signed in as {auth.account.email}.</p>
          <Button onClick={() => navigate("/")}>Continue to Horizon</Button>
        </>
      ) : auth.requestStatus === "sent" ? (
        <>
          <p role="status">
            If that address can receive mail, a sign-in link is on its way.
          </p>
          <small>For local development, check the Mailpit inbox.</small>
          <Button variant="outline" onClick={auth.resetRequest}>
            Use a different email
          </Button>
        </>
      ) : (
        <form onSubmit={submit}>
          <p>Sign in with a one-time link sent to your email.</p>
          <label className="form-field">
            Email address
            <input name="email" type="email" autoComplete="email" required />
          </label>
          {auth.error && <p role="alert">{auth.error}</p>}
          <Button disabled={auth.requestStatus === "sending"}>
            {auth.requestStatus === "sending"
              ? "Sending…"
              : "Send sign-in link"}
          </Button>
          <small className="form-footnote">
            Your sign-in credentials stay in memory and are cleared when you
            leave.
          </small>
        </form>
      )}
    </main>
  );
}
