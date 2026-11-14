ALTER TABLE spaces
    ADD COLUMN deleted_at TIMESTAMPTZ,
    ADD COLUMN deleted_by UUID REFERENCES users(id),
    ADD CONSTRAINT spaces_deleted_metadata_check
        CHECK ((deleted_at IS NULL) = (deleted_by IS NULL));

CREATE INDEX spaces_deleted_by_deleted_at_idx
    ON spaces (deleted_by, deleted_at)
    WHERE deleted_at IS NOT NULL;

CREATE TABLE deleted_space_memberships (
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    membership_created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (space_id, user_id)
);
