ALTER TABLE connections ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

UPDATE connections
SET sort_order = (
    SELECT COUNT(*)
    FROM connections AS c2
    WHERE (
        (c2.workspace_id = connections.workspace_id)
        OR (c2.workspace_id IS NULL AND connections.workspace_id IS NULL)
    )
    AND (
        c2.updated_at > connections.updated_at
        OR (c2.updated_at = connections.updated_at AND c2.id < connections.id)
    )
) + 1
WHERE sort_order = 0;

CREATE INDEX IF NOT EXISTS idx_connections_sort_order ON connections(workspace_id, sort_order);
