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
