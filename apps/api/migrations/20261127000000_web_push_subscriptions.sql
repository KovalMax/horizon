CREATE TABLE web_push_subscriptions (
    endpoint TEXT PRIMARY KEY CHECK (length(endpoint) BETWEEN 1 AND 2048),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    p256dh BYTEA NOT NULL CHECK (octet_length(p256dh) = 65),
    auth_secret BYTEA NOT NULL CHECK (octet_length(auth_secret) = 16),
    calendar_reminders BOOLEAN NOT NULL,
    task_reminders BOOLEAN NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX web_push_subscriptions_user_idx ON web_push_subscriptions (user_id);
