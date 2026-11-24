CREATE TABLE calendar_reminder_preferences (
    event_id UUID NOT NULL REFERENCES calendar_events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    offsets_minutes INTEGER[] NOT NULL CHECK (
        cardinality(offsets_minutes) BETWEEN 1 AND 5
        AND offsets_minutes <@ ARRAY[15, 30, 60, 120, 1440, 2880, 10080]::INTEGER[]
    ),
    PRIMARY KEY (event_id, user_id)
);

CREATE TABLE calendar_reminder_deliveries (
    event_id UUID NOT NULL REFERENCES calendar_events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    occurrence_date DATE NOT NULL,
    offset_minutes INTEGER NOT NULL,
    PRIMARY KEY (event_id, user_id, occurrence_date, offset_minutes)
);
