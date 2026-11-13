import type { PrototypeAppState } from "./context";
import { Button, HorizonMark } from "../../../shared/ui";

export function WelcomePage({ state }: { state: PrototypeAppState }) {
  const { showToast } = state;
  return (
    <section className="welcome-card">
      <HorizonMark large />
      <span className="eyebrow">WELCOME TO HORIZON</span>
      <h1>Your life, across Spaces.</h1>
      <p>Sign in with a link sent to your email. No password needed.</p>
      <label className="form-field">
        Email address
        <input type="email" placeholder="you@example.com" />
      </label>
      <Button
        onClick={() =>
          showToast("Sign-in link sent (simulated). No email was sent.")
        }
      >
        Send sign-in link
      </Button>
      <small className="form-footnote">
        Simulated prototype · no account or email is created.
      </small>
    </section>
  );
}
