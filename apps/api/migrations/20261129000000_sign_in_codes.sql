ALTER TABLE auth_challenges
    ADD COLUMN code_hash BYTEA,
    ADD COLUMN code_attempts INTEGER NOT NULL DEFAULT 0 CHECK (code_attempts BETWEEN 0 AND 5);
