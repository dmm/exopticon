-- Your SQL goes here

-- convert offsets from milliseconds to microseconds
ALTER TABLE observations
ALTER COLUMN frame_offset SET DATA TYPE bigint;

UPDATE observations SET frame_offset = frame_offset * 1000;
