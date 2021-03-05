-- This file should undo anything in `up.sql`

DELETE FROM analysis_instances
WHERE analysis_engine_id IN (2, 3);
