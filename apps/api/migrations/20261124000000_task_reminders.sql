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
