-- Your SQL goes here

-- create pgcrypt extension for the gen_random_uuid() function
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Add uuid column with default values
ALTER TABLE video_units
ADD COLUMN newid uuid DEFAULT gen_random_uuid();

-- Add column for new id in observations
ALTER TABLE observations
ADD COLUMN new_video_unit_id uuid;

-- Populate uuids from video_units
UPDATE observations
SET new_video_unit_id = video_units.newid
FROM video_units
WHERE
  observations.video_unit_id = video_units.id
;

-- add column for new id in video_files
ALTER TABLE video_files
ADD COLUMN new_video_unit_id uuid;

-- populate uuids from video_units
UPDATE video_files
SET new_video_unit_id = video_units.newid
FROM video_units
WHERE
  video_files.video_unit_id = video_units.id
;

-- Drop foreign key constaints for video_units.id
ALTER TABLE observations
DROP CONSTRAINT observations_video_unit_id_fkey;

ALTER TABLE video_files
DROP CONSTRAINT video_files_video_unit_id_fkey;

-- DROP video_unit_id column
ALTER TABLE observations
DROP COLUMN video_unit_id;

ALTER TABLE video_files
DROP COLUMN video_unit_id;

-- rename new_video_unit_id column
ALTER TABLE observations
RENAME COLUMN new_video_unit_id to video_unit_id;

ALTER TABLE video_files
RENAME COLUMN new_video_unit_id to video_unit_id;

-- make foreign keys non nullable
ALTER TABLE observations
ALTER COLUMN video_unit_id SET NOT NULL
;

ALTER TABLE video_files
ALTER COLUMN video_unit_id SET NOT NULL
;

-- make newid primary key
ALTER TABLE video_units
DROP CONSTRAINT video_units_pkey;

ALTER TABLE video_units
DROP COLUMN id;

ALTER TABLE video_units
RENAME COLUMN newid TO id;

ALTER TABLE video_units
ALTER COLUMN id SET NOT NULL
;

ALTER TABLE video_units
ADD PRIMARY KEY (id);

-- set the id column non-null and remove default
ALTER TABLE video_units
ALTER COLUMN id SET NOT NULL;

ALTER TABLE video_units
ALTER COLUMN id DROP DEFAULT;

-- Add foreign key relationships
ALTER TABLE observations
ADD FOREIGN KEY (video_unit_id) REFERENCES video_units (id);

ALTER TABLE video_files
ADD FOREIGN KEY (video_unit_id) REFERENCES video_units (id);

