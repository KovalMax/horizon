ALTER TABLE web_push_subscriptions
    ADD COLUMN session_id UUID REFERENCES auth_sessions(id) ON DELETE CASCADE;

CREATE TABLE web_push_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint TEXT NOT NULL REFERENCES web_push_subscriptions(endpoint) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    reminder_key TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('calendar', 'tasks')),
    item_id UUID NOT NULL,
    occurrence_date DATE,
    reference_time TIMESTAMPTZ NOT NULL,
    offset_minutes INTEGER NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    available_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempts INTEGER NOT NULL DEFAULT 0,
    finished BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (endpoint, reminder_key)
);

CREATE INDEX web_push_outbox_due_idx ON web_push_outbox(available_at) WHERE NOT finished;
