-- This file should undo anything in `up.sql`
ALTER TABLE camera_groups
DROP CONSTRAINT max_name_length;
