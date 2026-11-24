ALTER TABLE tasks
    ALTER COLUMN reminder_offsets_minutes SET DEFAULT '{}',
    DROP CONSTRAINT tasks_reminder_offsets_valid,
    ADD CONSTRAINT tasks_reminder_offsets_valid CHECK (
        cardinality(reminder_offsets_minutes) <= 5
        AND reminder_offsets_minutes <@ ARRAY[15, 30, 60, 120, 1440, 2880, 10080]::INTEGER[]
    );

ALTER TABLE task_reminder_preferences
    DROP CONSTRAINT task_reminder_preferences_offsets_minutes_check,
    ADD CONSTRAINT task_reminder_preferences_offsets_minutes_check CHECK (
        cardinality(offsets_minutes) <= 5
        AND offsets_minutes <@ ARRAY[15, 30, 60, 120, 1440, 2880, 10080]::INTEGER[]
    );

ALTER TABLE task_reminder_jobs
    DROP CONSTRAINT task_reminder_jobs_offset_minutes_check,
    ADD CONSTRAINT task_reminder_jobs_offset_minutes_check CHECK (
        offset_minutes IN (15, 30, 60, 120, 1440, 2880, 10080)
    );
