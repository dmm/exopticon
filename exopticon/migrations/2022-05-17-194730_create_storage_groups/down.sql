-- This file should undo anything in `up.sql`

ALTER TABLE cameras RENAME COLUMN storage_group_id to camera_group_id;

ALTER TABLE storage_groups RENAME TO camera_groups;

