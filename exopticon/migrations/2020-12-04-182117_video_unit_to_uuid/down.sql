-- This file should undo anything in `up.sql`

-- Add SERIAL oldid column
ALTER TABLE video_units
ADD COLUMN oldid SERIAL;

-- Add oldid column to child tables
ALTER TABLE video_files
ADD COLUMN old_video_unit_id INTEGER;

ALTER TABLE observations
ADD COLUMN old_video_unit_id INTEGER;

-- populate foreign keys for child tables
UPDATE video_files
SET old_video_unit_id = video_units.oldid
FROM video_units
WHERE
  video_files.video_unit_id = video_units.id
;

UPDATE observations
SET old_video_unit_id = video_units.oldid
FROM video_units
WHERE
  observations.video_unit_id = video_units.id
;

-- Drop foreign key constraints on child tables
ALTER TABLE observations
DROP CONSTRAINT observations_video_unit_id_fkey;

ALTER TABLE video_files
DROP CONSTRAINT video_files_video_unit_id_fkey;

-- Drop new primary key column
ALTER TABLE video_units
DROP COLUMN id
;

ALTER TABLE video_units
RENAME COLUMN oldid TO id;

ALTER TABLE video_units
ADD PRIMARY KEY (id);

-- Drop foreign key column in child tables
ALTER TABLE observations
DROP COLUMN video_unit_id;

ALTER TABLE video_files
DROP COLUMN video_unit_id;

-- Rename foreign keys in child tables
ALTER TABLE observations
RENAME COLUMN old_video_unit_id TO video_unit_id
;

ALTER TABLE video_files
RENAME COLUMN old_video_unit_id TO video_unit_id
;

-- Add foreign key constraints
ALTER TABLE video_files
ADD FOREIGN KEY (video_unit_id) REFERENCES video_units (id);

ALTER TABLE observations
ADD FOREIGN KEY (video_unit_id) REFERENCES video_units (id);

-- Add not null constraints on child tables
ALTER TABLE video_files
ALTER COLUMN video_unit_id SET NOT NULL
;

ALTER TABLE observations
ALTER COLUMN video_unit_id SET NOT NULL
;
