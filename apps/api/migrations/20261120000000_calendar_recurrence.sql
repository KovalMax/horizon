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
