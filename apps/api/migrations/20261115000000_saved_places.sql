CREATE TABLE saved_places (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    category TEXT NOT NULL CHECK (category IN ('Food & drink', 'Groceries', 'Health', 'Services')),
    address TEXT NOT NULL CHECK (length(address) BETWEEN 1 AND 300),
    note TEXT CHECK (note IS NULL OR length(note) <= 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX saved_places_space_category_idx ON saved_places (space_id, category, name);
