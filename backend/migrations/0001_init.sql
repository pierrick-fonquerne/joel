-- Joel database baseline. Auth tables arrive with migration 0002.
CREATE TABLE schema_marker (
    id INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    installed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO schema_marker (id) VALUES (1);
