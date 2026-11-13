CREATE TABLE todo_lists (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (space_id, name)
);

CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    list_id UUID NOT NULL REFERENCES todo_lists(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 160),
    due_date DATE,
    due_time TIME,
    done BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX todo_lists_space_idx ON todo_lists (space_id, name);
CREATE INDEX tasks_list_created_idx ON tasks (list_id, created_at);
