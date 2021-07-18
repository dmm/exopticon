-- This file should undo anything in `up.sql`

DELETE FROM analysis_engines
WHERE id = 4;

DROP TABLE observation_snapshots;

DROP INDEX event_observations_event_id_idx;

DROP TABLE event_observations;

DROP TABLE events;
