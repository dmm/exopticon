-- This file should undo anything in `up.sql`

UPDATE observations SET offset = offset / 1000;

ALTER TABLE observations
ALTER COLUMN frame_offset SET DATA TYPE int;
