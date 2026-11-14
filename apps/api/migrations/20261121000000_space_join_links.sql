CREATE TABLE space_join_links (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    CHECK (expires_at IS NULL OR expires_at > created_at)
);

CREATE INDEX space_join_links_space_created_idx
    ON space_join_links (space_id, created_at DESC);

CREATE TABLE space_join_requests (
    id UUID PRIMARY KEY,
    link_id UUID NOT NULL REFERENCES space_join_links(id),
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    requester_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requested_role TEXT NOT NULL CHECK (requested_role IN ('viewer', 'editor')),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'declined')),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at TIMESTAMPTZ,
    decided_by UUID REFERENCES users(id),
    CHECK (
        (status = 'pending' AND decided_at IS NULL AND decided_by IS NULL)
        OR (status IN ('approved', 'declined') AND decided_at IS NOT NULL AND decided_by IS NOT NULL)
    )
);

CREATE UNIQUE INDEX space_join_requests_one_pending_per_account_space_idx
    ON space_join_requests (space_id, requester_id) WHERE status = 'pending';

CREATE INDEX space_join_requests_space_pending_idx
    ON space_join_requests (space_id, requested_at DESC) WHERE status = 'pending';

CREATE INDEX space_join_requests_requester_idx
    ON space_join_requests (requester_id, requested_at DESC);
