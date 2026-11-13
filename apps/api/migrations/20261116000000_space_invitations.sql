CREATE TABLE space_invitations (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    invited_by UUID NOT NULL REFERENCES users(id),
    email_normalized TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK (role = 'viewer'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);

CREATE INDEX space_invitations_email_pending_idx
    ON space_invitations (email_normalized, expires_at)
    WHERE accepted_at IS NULL;
CREATE UNIQUE INDEX space_invitations_one_pending_email_idx
    ON space_invitations (space_id, email_normalized)
    WHERE accepted_at IS NULL;
