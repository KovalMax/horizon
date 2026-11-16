ALTER TABLE auth_challenges
    ADD COLUMN time_zone TEXT NOT NULL DEFAULT 'UTC';
