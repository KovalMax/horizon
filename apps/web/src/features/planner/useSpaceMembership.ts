import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import type { Space } from "./domain";
import {
  acceptSpaceInvitation,
  createSpaceInvitation,
  listMyInvitations,
  listSpaceInvitations,
  listSpaceMembers,
  updateSpaceMemberRole,
  removeSpaceMember,
  revokeSpaceInvitation,
  createSpaceJoinLink,
  listSpaceJoinLinks,
  revokeSpaceJoinLink,
  listPendingSpaceJoinRequests,
  decideSpaceJoinRequest,
  requestSpaceJoin,
  listMySpaceJoinRequests,
  MembersApiError,
  type SpaceInvitation,
  type SpaceJoinLink,
  type SpaceJoinRequest,
  type SpaceMember,
} from "./membersApi";
import {
  deleteSpace as deleteSpaceApi,
  restoreSpace as restoreSpaceApi,
  transferSpaceOwnership as transferSpaceOwnershipApi,
  SpacesApiError,
  type DeletedSpace,
} from "./spacesApi";

type SpaceMembershipOptions = {
  accessToken: string;
  currentSpaceId: string;
  serverSpaces: Space[];
  setServerSpaces: Dispatch<SetStateAction<Space[]>>;
  setSpacesReload: Dispatch<SetStateAction<number>>;
  setSpacesError: Dispatch<SetStateAction<string>>;
  setDeletedSpaces: Dispatch<SetStateAction<DeletedSpace[]>>;
  go: (path: string) => void;
  showToast: (message: string) => void;
};

export function useSpaceMembership({
  accessToken,
  currentSpaceId,
  serverSpaces,
  setServerSpaces,
  setSpacesReload,
  setSpacesError,
  setDeletedSpaces,
  go,
  showToast,
}: SpaceMembershipOptions) {
  const [spaceMembers, setSpaceMembers] = useState<SpaceMember[]>([]);
  const [spaceInvitations, setSpaceInvitations] = useState<SpaceInvitation[]>(
    [],
  );
  const [spaceJoinLinks, setSpaceJoinLinks] = useState<SpaceJoinLink[]>([]);
  const [pendingJoinRequests, setPendingJoinRequests] = useState<
    SpaceJoinRequest[]
  >([]);
  const [myJoinRequests, setMyJoinRequests] = useState<SpaceJoinRequest[]>([]);
  const [myInvitations, setMyInvitations] = useState<SpaceInvitation[]>([]);
  const [membershipStatus, setMembershipStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [membershipError, setMembershipError] = useState("");
  const [membershipReload, setMembershipReload] = useState(0);
  const [savingInvitation, setSavingInvitation] = useState(false);
  const [savingJoinLink, setSavingJoinLink] = useState(false);
  const [acceptingInvitation, setAcceptingInvitation] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    const canManageCurrentSpace = serverSpaces.some(
      (item) => item.id === currentSpaceId && item.role === "Owner",
    );
    setMembershipStatus("loading");
    setMembershipError("");
    Promise.all([
      currentSpaceId
        ? listSpaceMembers(accessToken, currentSpaceId)
        : Promise.resolve([]),
      currentSpaceId
        ? canManageCurrentSpace
          ? listSpaceInvitations(accessToken, currentSpaceId).catch(() => [])
          : Promise.resolve([])
        : Promise.resolve([]),
      listMyInvitations(accessToken),
      currentSpaceId && canManageCurrentSpace
        ? listSpaceJoinLinks(accessToken, currentSpaceId)
        : Promise.resolve([]),
      currentSpaceId && canManageCurrentSpace
        ? listPendingSpaceJoinRequests(accessToken, currentSpaceId)
        : Promise.resolve([]),
      listMySpaceJoinRequests(accessToken),
    ])
      .then(([members, invitations, mine, links, requests, mineRequests]) => {
        if (controller.signal.aborted) return;
        setSpaceMembers(members);
        setSpaceInvitations(invitations);
        setMyInvitations(mine);
        setSpaceJoinLinks(links);
        setPendingJoinRequests(requests);
        setMyJoinRequests(mineRequests);
        setMembershipStatus("ready");
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setMembershipStatus("error");
        setMembershipError(
          "Space Members couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => controller.abort();
  }, [accessToken, currentSpaceId, membershipReload, serverSpaces]);

  const inviteMember = async (email: string) => {
    if (!currentSpaceId) return false;
    setSavingInvitation(true);
    setMembershipError("");
    try {
      const invitation = await createSpaceInvitation(
        accessToken,
        currentSpaceId,
        email,
      );
      setSpaceInvitations((all) => [invitation, ...all]);
      showToast(`Invitation sent to ${invitation.email}.`);
      return true;
    } catch (error) {
      setMembershipError(
        error instanceof MembersApiError && error.status === 403
          ? "Only Owners can invite people."
          : error instanceof MembersApiError && error.status === 409
            ? "This email is already a Member or has a pending invitation."
            : "The invitation couldn’t be sent. Check the email address and try again.",
      );
      return false;
    } finally {
      setSavingInvitation(false);
    }
  };

  const changeMemberRole = async (
    userId: string,
    role: "editor" | "viewer",
  ) => {
    if (!currentSpaceId) return false;
    try {
      const updated = await updateSpaceMemberRole(
        accessToken,
        currentSpaceId,
        userId,
        role,
      );
      setSpaceMembers((all) =>
        all.map((member) => (member.user_id === userId ? updated : member)),
      );
      showToast("Member role updated.");
      return true;
    } catch {
      setMembershipError(
        "The Member role couldn’t be changed. Refresh and try again.",
      );
      return false;
    }
  };

  const removeMember = async (userId: string) => {
    if (!currentSpaceId) return false;
    try {
      await removeSpaceMember(accessToken, currentSpaceId, userId);
      setSpaceMembers((all) =>
        all.filter((member) => member.user_id !== userId),
      );
      setServerSpaces((all) =>
        all.map((item) =>
          item.id === currentSpaceId
            ? { ...item, members: Math.max(0, item.members - 1) }
            : item,
        ),
      );
      showToast("Member removed from the Space.");
      return true;
    } catch {
      setMembershipError(
        "The Member couldn’t be removed. Refresh and try again.",
      );
      return false;
    }
  };

  const revokeInvitation = async (invitationId: string) => {
    if (!currentSpaceId) return false;
    try {
      await revokeSpaceInvitation(accessToken, currentSpaceId, invitationId);
      setSpaceInvitations((all) =>
        all.filter((item) => item.id !== invitationId),
      );
      showToast(
        "Invitation revoked. You can safely invite this address again.",
      );
      return true;
    } catch {
      setMembershipError(
        "The invitation couldn’t be revoked. Refresh and try again.",
      );
      return false;
    }
  };

  const createJoinLink = async (
    role: "viewer" | "editor",
    expiresAt: string | null,
  ) => {
    if (!currentSpaceId) return false;
    setSavingJoinLink(true);
    setMembershipError("");
    try {
      const link = await createSpaceJoinLink(
        accessToken,
        currentSpaceId,
        role,
        expiresAt,
      );
      setSpaceJoinLinks((all) => [link, ...all]);
      showToast("Join link created. Copy it to share access.");
      return true;
    } catch {
      setMembershipError(
        "The join link couldn’t be created. Check its expiry and try again.",
      );
      return false;
    } finally {
      setSavingJoinLink(false);
    }
  };

  const revokeJoinLink = async (linkId: string) => {
    if (!currentSpaceId) return false;
    try {
      await revokeSpaceJoinLink(accessToken, currentSpaceId, linkId);
      const revokedAt = new Date().toISOString();
      setSpaceJoinLinks((all) =>
        all.map((link) =>
          link.id === linkId ? { ...link, revoked_at: revokedAt } : link,
        ),
      );
      showToast("Join link revoked. It can no longer be used.");
      return true;
    } catch {
      setMembershipError(
        "The join link couldn’t be revoked. Refresh and try again.",
      );
      return false;
    }
  };

  const decideJoinRequest = async (
    requestId: string,
    decision: "approved" | "declined",
  ) => {
    if (!currentSpaceId) return false;
    try {
      const result = await decideSpaceJoinRequest(
        accessToken,
        currentSpaceId,
        requestId,
        decision,
      );
      setPendingJoinRequests((all) =>
        all.filter((item) => item.id !== requestId),
      );
      setMyJoinRequests((all) =>
        all.map((item) => (item.id === result.id ? result : item)),
      );
      if (decision === "approved") {
        setMembershipReload((current) => current + 1);
        setSpacesReload((current) => current + 1);
      }
      showToast(
        decision === "approved"
          ? "Join request approved."
          : "Join request declined.",
      );
      return true;
    } catch {
      setMembershipError(
        "The join request couldn’t be updated. Refresh and try again.",
      );
      return false;
    }
  };

  const requestJoinAccess = async (linkId: string) => {
    try {
      const result = await requestSpaceJoin(accessToken, linkId);
      setMyJoinRequests((all) => [
        result,
        ...all.filter((item) => item.id !== result.id),
      ]);
      return result;
    } catch {
      return null;
    }
  };

  const transferOwnership = async (newOwnerId: string) => {
    if (!currentSpaceId) return false;
    try {
      await transferSpaceOwnershipApi(accessToken, currentSpaceId, newOwnerId);
      setSpacesReload((current) => current + 1);
      setMembershipReload((current) => current + 1);
      showToast("Ownership transferred. You now have Editor access.");
      return true;
    } catch (error) {
      setMembershipError(
        error instanceof SpacesApiError && error.status === 403
          ? "Only the current Owner can transfer this Space."
          : "Ownership couldn’t be transferred. Refresh and try again.",
      );
      return false;
    }
  };

  const deleteCurrentSpace = async () => {
    if (!currentSpaceId) return false;
    try {
      const deleted = await deleteSpaceApi(accessToken, currentSpaceId);
      setSpacesReload((current) => current + 1);
      go("/");
      showToast(
        `${deleted.name} was deleted. You can restore it for seven days.`,
      );
      return true;
    } catch (error) {
      setMembershipError(
        error instanceof SpacesApiError && error.status === 403
          ? "Only the current Owner can delete this Space."
          : "The Space couldn’t be deleted. Refresh and try again.",
      );
      return false;
    }
  };

  const restoreDeletedSpace = async (spaceId: string) => {
    try {
      const restored = await restoreSpaceApi(accessToken, spaceId);
      setDeletedSpaces((all) => all.filter((item) => item.id !== spaceId));
      setSpacesReload((current) => current + 1);
      go(`/spaces/${restored.id}/calendar`);
      showToast(
        `${restored.name} was restored. Previous invitations and links remain invalid.`,
      );
      return true;
    } catch {
      setSpacesError(
        "This Space can no longer be restored. Refresh and try again.",
      );
      return false;
    }
  };

  const acceptInvitation = async (invitationId: string) => {
    setAcceptingInvitation(invitationId);
    setMembershipError("");
    try {
      const accepted = await acceptSpaceInvitation(accessToken, invitationId);
      setMyInvitations((all) =>
        all.filter((invitation) => invitation.id !== invitationId),
      );
      setMembershipReload((current) => current + 1);
      setSpacesReload((current) => current + 1);
      showToast(`You joined ${accepted.space_name} as a Viewer.`);
    } catch {
      setMembershipError(
        "This invitation can’t be accepted with the signed-in email.",
      );
    } finally {
      setAcceptingInvitation("");
    }
  };

  return {
    spaceMembers,
    spaceInvitations,
    spaceJoinLinks,
    pendingJoinRequests,
    myJoinRequests,
    myInvitations,
    membershipStatus,
    membershipError,
    setMembershipError,
    savingInvitation,
    savingJoinLink,
    acceptingInvitation,
    inviteMember,
    changeMemberRole,
    removeMember,
    revokeInvitation,
    createJoinLink,
    revokeJoinLink,
    decideJoinRequest,
    requestJoinAccess,
    transferOwnership,
    deleteCurrentSpace,
    restoreDeletedSpace,
    acceptInvitation,
  };
}
