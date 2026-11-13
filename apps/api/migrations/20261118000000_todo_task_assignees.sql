ALTER TABLE tasks
    ADD COLUMN assignee_id UUID REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX tasks_assignee_idx
    ON tasks (assignee_id)
    WHERE assignee_id IS NOT NULL;
