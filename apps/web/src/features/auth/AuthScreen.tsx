import { useEffect, useRef, useState, type FormEvent } from "react";
import type { AuthState } from "./useAuth";
import { Button, HorizonMark } from "../../shared/ui";
import type { SpaceJoinRequest } from "../planner/membersApi";

export function AuthScreen({
  path,
  auth,
  navigate,
  joinRequests = [],
  requestJoinAccess = async () => null,
}: {
  path: string;
  auth: AuthState;
  navigate: (path: string) => void;
  joinRequests?: SpaceJoinRequest[];
  requestJoinAccess?: (linkId: string) => Promise<SpaceJoinRequest | null>;
}) {
  const handledCallback = useRef(false);
  const [missingCallbackToken, setMissingCallbackToken] = useState(false);
  const joinToken = new URLSearchParams(window.location.search).get("join");
  const [joinSubmission, setJoinSubmission] = useState<SpaceJoinRequest | null>(
    null,
  );
  const [joinError, setJoinError] = useState("");

  useEffect(() => {
    if (path !== "/welcome") return;
    if (joinToken) window.localStorage.setItem("horizon-join-link", joinToken);
    else window.localStorage.removeItem("horizon-join-link");
  }, [joinToken, path]);

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
    if (!token) {
      if (auth.tokens) navigate("/");
      else if (auth.redemptionStatus !== "redeeming") {
        setMissingCallbackToken(true);
      }
      return;
    }
    const savedJoinToken = window.localStorage.getItem("horizon-join-link");
    void auth.redeemLink(token).then((succeeded) => {
      if (!succeeded) return;
      if (savedJoinToken) {
        window.localStorage.removeItem("horizon-join-link");
        navigate(`/welcome?join=${encodeURIComponent(savedJoinToken)}`);
      } else {
        navigate("/");
      }
    });
  }, [auth, navigate, path]);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const email = new FormData(event.currentTarget).get("email");
    if (typeof email === "string") void auth.requestLink(email.trim());
  };

  if (path === "/auth/complete") {
    const redemptionFailed =
      auth.redemptionStatus === "failed" || missingCallbackToken;
    return (
      <main className="welcome-card auth-card" aria-live="polite">
        <HorizonMark large />
        <span className="eyebrow">SECURE SIGN-IN</span>
        <h1>{redemptionFailed ? "Link not accepted" : "Signing you in…"}</h1>
        {auth.redemptionStatus === "redeeming" && (
          <p role="status">Verifying your sign-in link.</p>
        )}
        {missingCallbackToken ? (
          <p role="alert">
            This sign-in link is missing its token. Request a new link.
          </p>
        ) : (
          auth.error && <p role="alert">{auth.error}</p>
        )}
        {redemptionFailed && (
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
          {joinToken && (
            <section aria-label="Join Space">
              <h2>Request to join this Space</h2>
              {joinSubmission?.status === "pending" ||
              joinRequests.some(
                (item) =>
                  item.link_id === joinToken && item.status === "pending",
              ) ? (
                <p role="status">Your request is pending Owner approval.</p>
              ) : joinSubmission?.status === "approved" ||
                joinRequests.some(
                  (item) =>
                    item.link_id === joinToken && item.status === "approved",
                ) ? (
                <p role="status">
                  Your request was approved. You now have access.
                </p>
              ) : (
                <>
                  <p>
                    The Space Owner must approve your request before you can
                    join.
                  </p>
                  {joinError && <p role="alert">{joinError}</p>}
                  <Button
                    onClick={async () => {
                      setJoinError("");
                      const result = await requestJoinAccess(joinToken);
                      if (result) setJoinSubmission(result);
                      else
                        setJoinError(
                          "This link may have expired or been revoked. Ask the Space Owner for a new link.",
                        );
                    }}
                  >
                    Request access
                  </Button>
                </>
              )}
            </section>
          )}
          {joinRequests.length > 0 && (
            <section aria-label="Your join requests">
              <h2>Your join requests</h2>
              {joinRequests.map((item) => (
                <p key={item.id}>
                  {item.space_name}:{" "}
                  {item.status === "approved"
                    ? "Approved"
                    : item.status === "declined"
                      ? "Declined"
                      : "Pending"}{" "}
                  ({item.role})
                </p>
              ))}
            </section>
          )}
          <Button onClick={() => navigate("/")}>Continue to Horizon</Button>
        </>
      ) : auth.requestStatus === "sent" ? (
        <>
          {joinToken && (
            <p>After signing in, return to this link to request access.</p>
          )}
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
          <p>
            {joinToken
              ? "Sign in to request access to this Space. The Owner must approve your request."
              : "Sign in with a one-time link sent to your email."}
          </p>
          <label className="form-field">
            Email address
            <input name="email" type="email" autoComplete="email" required />
          </label>
          {auth.error && <p role="alert">{auth.error}</p>}
          <Button type="submit" disabled={auth.requestStatus === "sending"}>
            {auth.requestStatus === "sending"
              ? "Sending…"
              : "Send sign-in link"}
          </Button>
          <small className="form-footnote">
            You’ll stay signed in when you reload this tab. Sign out to end your
            session.
          </small>
        </form>
      )}
    </main>
  );
}
