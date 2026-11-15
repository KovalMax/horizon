-- Consolidated baseline schema. Historical migrations are applied in version order.

-- 20261110000000_authentication.sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE email_identities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email_normalized TEXT NOT NULL UNIQUE,
    verified_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE auth_challenges (
    id UUID PRIMARY KEY,
    email_normalized TEXT NOT NULL,
    secret_hash BYTEA NOT NULL UNIQUE,
    request_ip TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);

CREATE INDEX auth_challenges_email_created_idx
    ON auth_challenges (email_normalized, created_at DESC);
CREATE INDEX auth_challenges_ip_created_idx
    ON auth_challenges (request_ip, created_at DESC);

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    idle_expires_at TIMESTAMPTZ NOT NULL,
    absolute_expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    CHECK (idle_expires_at <= absolute_expires_at)
);

CREATE INDEX auth_sessions_user_created_idx ON auth_sessions (user_id, created_at DESC);

CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES auth_sessions(id) ON DELETE CASCADE,
    secret_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);

CREATE INDEX refresh_tokens_session_created_idx ON refresh_tokens (session_id, created_at DESC);

CREATE TABLE spaces (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('personal', 'shared')),
    personal_owner_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((kind = 'personal' AND personal_owner_id IS NOT NULL) OR
           (kind = 'shared' AND personal_owner_id IS NULL))
);

CREATE TABLE space_memberships (
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (space_id, user_id)
);

CREATE UNIQUE INDEX space_memberships_one_owner_idx
    ON space_memberships (space_id) WHERE role = 'owner';

-- 20261113000000_calendar_events.sql
CREATE TABLE calendar_events (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 120),
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ,
    time_zone TEXT NOT NULL CHECK (length(time_zone) BETWEEN 1 AND 64),
    all_day BOOLEAN NOT NULL DEFAULT FALSE,
    location TEXT CHECK (location IS NULL OR length(location) <= 240),
    notes TEXT CHECK (notes IS NULL OR length(notes) <= 2000),
    recurrence TEXT CHECK (recurrence IS NULL OR recurrence IN ('daily', 'weekly', 'monthly')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ends_at IS NULL OR ends_at > starts_at)
);

CREATE INDEX calendar_events_space_start_idx ON calendar_events (space_id, starts_at);

-- 20261114000000_todo_lists.sql
CREATE TABLE todo_lists (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (space_id, name)
);

CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    list_id UUID NOT NULL REFERENCES todo_lists(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 160),
    due_date DATE,
    due_time TIME,
    done BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX todo_lists_space_idx ON todo_lists (space_id, name);
CREATE INDEX tasks_list_created_idx ON tasks (list_id, created_at);

-- 20261115000000_saved_places.sql
CREATE TABLE saved_places (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    category TEXT NOT NULL CHECK (category IN ('Food & drink', 'Groceries', 'Health', 'Services')),
    address TEXT NOT NULL CHECK (length(address) BETWEEN 1 AND 300),
    note TEXT CHECK (note IS NULL OR length(note) <= 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX saved_places_space_category_idx ON saved_places (space_id, category, name);

-- 20261116000000_space_invitations.sql
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

-- 20261117000000_sql_review_indexes_and_invitation_status.sql
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

-- 20261118000000_todo_task_assignees.sql
ALTER TABLE tasks
    ADD COLUMN assignee_id UUID REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX tasks_assignee_idx
    ON tasks (assignee_id)
    WHERE assignee_id IS NOT NULL;

-- 20261119000000_membership_administration.sql
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

-- 20261120000000_calendar_recurrence.sql
ALTER TABLE calendar_events
    ADD COLUMN recurrence_ends_on DATE,
    ADD CONSTRAINT calendar_events_recurrence_end_check
        CHECK (recurrence IS NOT NULL OR recurrence_ends_on IS NULL);

CREATE TABLE calendar_event_occurrence_overrides (
    event_id UUID NOT NULL REFERENCES calendar_events(id) ON DELETE CASCADE,
    occurrence_date DATE NOT NULL,
    cancelled BOOLEAN NOT NULL DEFAULT FALSE,
    title TEXT,
    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,
    time_zone TEXT,
    all_day BOOLEAN,
    location TEXT,
    notes TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, occurrence_date),
    CHECK (
        (cancelled AND title IS NULL AND starts_at IS NULL AND time_zone IS NULL AND all_day IS NULL)
        OR (NOT cancelled AND title IS NOT NULL AND starts_at IS NOT NULL
            AND time_zone IS NOT NULL AND all_day IS NOT NULL)
    )
);

-- 20261121000000_space_join_links.sql
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

-- 20261122000000_space_lifecycle.sql
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

-- 20261123000000_notifications.sql
CREATE TABLE notification_preferences (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'task_assignment', 'membership_change', 'event_change',
        'join_request', 'join_request_result'
    )),
    channel TEXT NOT NULL CHECK (channel IN ('in_app', 'email')),
    enabled BOOLEAN NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, event_type, channel)
);

CREATE TABLE space_notification_preferences (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    muted BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, space_id)
);

CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'task_assignment', 'membership_change', 'event_change',
        'join_request', 'join_request_result'
    )),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    destination TEXT NOT NULL CHECK (destination IN ('tasks', 'calendar', 'members')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ
);

CREATE INDEX notifications_recipient_created_idx
    ON notifications (recipient_id, created_at DESC, id DESC);
CREATE INDEX notifications_unread_recipient_idx
    ON notifications (recipient_id, created_at DESC)
    WHERE read_at IS NULL;

-- 20261124000000_task_reminders.sql
ALTER TABLE users
    ADD COLUMN time_zone TEXT NOT NULL DEFAULT 'UTC',
    ADD COLUMN default_reminder_time TIME NOT NULL DEFAULT '09:00';

ALTER TABLE tasks
    ADD COLUMN reminder_offsets_minutes INTEGER[] NOT NULL DEFAULT ARRAY[1440, 60],
    ADD CONSTRAINT tasks_reminder_offsets_valid CHECK (
        reminder_offsets_minutes <@ ARRAY[1440, 60]::INTEGER[]
    );

ALTER TABLE notification_preferences
    DROP CONSTRAINT notification_preferences_event_type_check,
    ADD CONSTRAINT notification_preferences_event_type_check CHECK (event_type IN (
        'task_assignment', 'membership_change', 'event_change',
        'join_request', 'join_request_result', 'due_reminder'
    ));

ALTER TABLE notifications
    DROP CONSTRAINT notifications_event_type_check,
    ADD CONSTRAINT notifications_event_type_check CHECK (event_type IN (
        'task_assignment', 'membership_change', 'event_change',
        'join_request', 'join_request_result', 'due_reminder'
    ));

CREATE TABLE task_reminder_preferences (
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    offsets_minutes INTEGER[] NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, user_id),
    CHECK (offsets_minutes <@ ARRAY[1440, 60]::INTEGER[])
);

CREATE TABLE task_reminder_jobs (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    offset_minutes INTEGER NOT NULL CHECK (offset_minutes IN (1440, 60)),
    scheduled_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sending', 'sent', 'failed', 'cancelled')),
    attempts INTEGER NOT NULL DEFAULT 0,
    available_at TIMESTAMPTZ NOT NULL,
    locked_at TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (task_id, recipient_id, offset_minutes)
);

CREATE INDEX task_reminder_jobs_due_idx ON task_reminder_jobs (available_at, scheduled_at)
    WHERE status IN ('pending', 'sending');

ALTER TABLE notifications
    ADD COLUMN reminder_job_id UUID UNIQUE REFERENCES task_reminder_jobs(id) ON DELETE SET NULL;
