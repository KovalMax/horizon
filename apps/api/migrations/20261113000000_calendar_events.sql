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
