import { useEffect, useState } from "react";
import {
  AuthApiError,
  getAuthTokens,
  getCurrentAccount,
  hasStoredRefreshToken,
  redeemMagicLink,
  requestMagicLink,
  restoreAuthSession,
  revokeCurrentSession,
  setAuthTokens,
  subscribeAuthTokens,
  type AuthAccount,
  type AuthTokens,
} from "./api";

function formatRetryDelay(seconds: number | null): string {
  if (seconds === null) return "a little while";
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 2) return "about a minute";
  if (minutes < 60) return `about ${minutes} minutes`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes
    ? `about ${hours} hour${hours === 1 ? "" : "s"} and ${remainingMinutes} minute${remainingMinutes === 1 ? "" : "s"}`
    : `about ${hours} hour${hours === 1 ? "" : "s"}`;
}

export type AuthState = {
  account: AuthAccount | null;
  tokens: AuthTokens | null;
  requestStatus: "idle" | "sending" | "sent";
  redemptionStatus: "idle" | "redeeming" | "failed";
  error: string;
  restoring: boolean;
  requestLink: (email: string, timeZone: string) => Promise<void>;
  redeemLink: (token: string) => Promise<boolean>;
  signOut: () => Promise<void>;
  resetRequest: () => void;
};

export function useAuth(): AuthState {
  const [account, setAccount] = useState<AuthAccount | null>(null);
  const [tokens, setTokens] = useState<AuthTokens | null>(null);
  const [requestStatus, setRequestStatus] =
    useState<AuthState["requestStatus"]>("idle");
  const [redemptionStatus, setRedemptionStatus] =
    useState<AuthState["redemptionStatus"]>("idle");
  const [error, setError] = useState("");
  const [restoring, setRestoring] = useState(hasStoredRefreshToken);

  useEffect(
    () =>
      subscribeAuthTokens((updatedTokens, reason) => {
        setTokens(updatedTokens);
        if (reason === "expired") {
          setAccount(null);
          setError("Your session expired. Please sign in again.");
        }
      }),
    [],
  );

  useEffect(() => {
    let active = true;
    const restore = async () => {
      try {
        const restoredTokens = await restoreAuthSession();
        if (!active) return;
        if (!restoredTokens) {
          if (hasStoredRefreshToken()) {
            setError(
              "We couldn’t restore your session. Check your connection and refresh this page to try again.",
            );
          }
          return;
        }
        const currentAccount = await getCurrentAccount(
          restoredTokens.accessToken,
        );
        if (!active) return;
        setTokens(getAuthTokens() ?? restoredTokens);
        setAccount(currentAccount);
      } catch {
        if (!active) return;
        setAccount(null);
        setError(
          "Your session is still available, but we couldn’t load your account. Check your connection and reload this tab to retry.",
        );
      } finally {
        if (active) setRestoring(false);
      }
    };
    void restore();
    return () => {
      active = false;
    };
  }, []);

  const requestLink = async (email: string, timeZone: string) => {
    setRequestStatus("sending");
    setError("");
    try {
      await requestMagicLink(email, timeZone);
      setRequestStatus("sent");
    } catch (failure) {
      setRequestStatus("idle");
      setError(
        failure instanceof AuthApiError && failure.status === 429
          ? `Too many sign-in requests were made recently. Please wait ${formatRetryDelay(failure.retryAfterSeconds)} before trying again. This does not affect your account.`
          : failure instanceof AuthApiError && failure.status === 503
            ? "Sign-in email is temporarily unavailable. Your account is safe; please try again later."
            : "We couldn’t send the sign-in link. Please check your connection and try again.",
      );
    }
  };

  const redeemLink = async (token: string): Promise<boolean> => {
    setRedemptionStatus("redeeming");
    setError("");
    let redeemedTokens: AuthTokens | null = null;
    try {
      const newTokens = await redeemMagicLink(token);
      redeemedTokens = newTokens;
      setAuthTokens(newTokens);
      const currentAccount = await getCurrentAccount(newTokens.accessToken);
      setTokens(getAuthTokens() ?? newTokens);
      setAccount(currentAccount);
      setRedemptionStatus("idle");
      return true;
    } catch (failure) {
      if (redeemedTokens) {
        setTokens(redeemedTokens);
      } else {
        setAuthTokens(null, "signed-out");
        setAccount(null);
      }
      setRedemptionStatus("failed");
      setError(
        redeemedTokens
          ? "Your sign-in succeeded, but we couldn’t load your account. Reload this tab to retry."
          : failure instanceof AuthApiError && failure.status === 401
            ? "This sign-in link is invalid, expired, or already used."
            : "We couldn't complete sign-in. Please try again shortly.",
      );
      return false;
    }
  };

  const signOut = async () => {
    const refreshToken = getAuthTokens()?.refreshToken ?? tokens?.refreshToken;
    if ("serviceWorker" in navigator) {
      void navigator.serviceWorker
        .getRegistration()
        .then((worker) => worker?.pushManager?.getSubscription())
        .then((subscription) => subscription?.unsubscribe())
        .catch(() => {});
    }
    setAuthTokens(null, "signed-out");
    setTokens(null);
    setAccount(null);
    setRequestStatus("idle");
    setRedemptionStatus("idle");
    setError("");
    if (!refreshToken) return;
    try {
      await revokeCurrentSession(refreshToken);
    } catch {
      setError("Signed out here, but the server couldn't revoke this session.");
    }
  };

  const resetRequest = () => {
    setRequestStatus("idle");
    setError("");
  };

  return {
    account,
    tokens,
    requestStatus,
    redemptionStatus,
    error,
    restoring,
    requestLink,
    redeemLink,
    signOut,
    resetRequest,
  };
}
