-- 0 = active, 1 = awaiting capture shutdown, 2 = ready for file cleanup.
ALTER TABLE cameras ADD COLUMN deletion_state INTEGER NOT NULL DEFAULT 0
    CHECK (deletion_state IN (0, 1, 2));

INSERT OR IGNORE INTO camera_groups (name, display_name) VALUES ('all', 'All');

INSERT INTO camera_group_memberships (camera_group_name, camera_name, display_order)
SELECT 'all', cameras.name,
    COALESCE((SELECT MAX(display_order) FROM camera_group_memberships
        WHERE camera_group_name = 'all'), -1) + ROW_NUMBER() OVER (ORDER BY cameras.name)
FROM cameras
WHERE NOT EXISTS (
    SELECT 1 FROM camera_group_memberships
    WHERE camera_group_name = 'all' AND camera_name = cameras.name
);
