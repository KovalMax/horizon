ALTER TABLE space_invitations
    DROP CONSTRAINT space_invitations_status_check,
    DROP CONSTRAINT space_invitations_status_accepted_at_check;

ALTER TABLE space_invitations
    ADD CONSTRAINT space_invitations_status_check
        CHECK (status IN ('pending', 'accepted', 'expired', 'revoked')),
    ADD CONSTRAINT space_invitations_status_accepted_at_check
        CHECK (
            (status = 'accepted' AND accepted_at IS NOT NULL)
            OR (status IN ('pending', 'expired', 'revoked') AND accepted_at IS NULL)
        );
