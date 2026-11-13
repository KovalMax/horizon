import { useState } from "react";
import {
  AuthApiError,
  getCurrentAccount,
  redeemMagicLink,
  requestMagicLink,
  revokeCurrentSession,
  type AuthAccount,
  type AuthTokens,
} from "./api";

export type AuthState = {
  account: AuthAccount | null;
  tokens: AuthTokens | null;
  requestStatus: "idle" | "sending" | "sent";
  redemptionStatus: "idle" | "redeeming" | "failed";
  error: string;
  requestLink: (email: string) => Promise<void>;
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

  const requestLink = async (email: string) => {
    setRequestStatus("sending");
    setError("");
    try {
      await requestMagicLink(email);
      setRequestStatus("sent");
    } catch {
      setRequestStatus("idle");
      setError("We couldn't send the sign-in link. Please try again shortly.");
    }
  };

  const redeemLink = async (token: string): Promise<boolean> => {
    setRedemptionStatus("redeeming");
    setError("");
    try {
      const newTokens = await redeemMagicLink(token);
      const currentAccount = await getCurrentAccount(newTokens.accessToken);
      setTokens(newTokens);
      setAccount(currentAccount);
      setRedemptionStatus("idle");
      return true;
    } catch (failure) {
      setTokens(null);
      setAccount(null);
      setRedemptionStatus("failed");
      setError(
        failure instanceof AuthApiError && failure.status === 401
          ? "This sign-in link is invalid, expired, or already used."
          : "We couldn't complete sign-in. Please try again shortly.",
      );
      return false;
    }
  };

  const signOut = async () => {
    const refreshToken = tokens?.refreshToken;
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
    requestLink,
    redeemLink,
    signOut,
    resetRequest,
  };
}
