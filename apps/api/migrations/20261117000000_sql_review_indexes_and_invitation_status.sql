ALTER TABLE space_invitations
    ADD COLUMN status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'expired'));

UPDATE space_invitations
SET status = CASE
    WHEN accepted_at IS NOT NULL THEN 'accepted'
    WHEN expires_at <= now() THEN 'expired'
    ELSE 'pending'
END;

ALTER TABLE space_invitations
    ADD CONSTRAINT space_invitations_status_accepted_at_check
    CHECK (
        (status = 'accepted' AND accepted_at IS NOT NULL)
        OR (status IN ('pending', 'expired') AND accepted_at IS NULL)
    );

DROP INDEX space_invitations_one_pending_email_idx;
CREATE UNIQUE INDEX space_invitations_one_pending_email_idx
    ON space_invitations (space_id, email_normalized)
    WHERE status = 'pending';

DROP INDEX space_invitations_email_pending_idx;
CREATE INDEX space_invitations_email_pending_idx
    ON space_invitations (email_normalized, expires_at)
    WHERE status = 'pending';

DROP INDEX todo_lists_space_idx;

CREATE INDEX space_memberships_user_space_idx
    ON space_memberships (user_id, space_id);

CREATE INDEX email_identities_user_created_idx
    ON email_identities (user_id, created_at);

DROP INDEX auth_sessions_user_created_idx;
CREATE INDEX auth_sessions_user_idx ON auth_sessions (user_id);

DROP INDEX refresh_tokens_session_created_idx;
CREATE INDEX refresh_tokens_session_idx ON refresh_tokens (session_id);
