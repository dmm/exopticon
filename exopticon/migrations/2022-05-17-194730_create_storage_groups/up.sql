-- Your SQL goes here

ALTER TABLE camera_groups RENAME TO storage_groups;

ALTER TABLE cameras RENAME COLUMN camera_group_id to storage_group_id;

