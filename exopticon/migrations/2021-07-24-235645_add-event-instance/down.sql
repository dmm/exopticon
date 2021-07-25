-- This file should undo anything in `up.sql`
DO $$
DECLARE event_instance_id Int;
BEGIN

SELECT id INTO event_instance_id
FROM analysis_instances
WHERE analysis_engine_id = 4
;

DELETE FROM analysis_subscriptions WHERE analysis_instance_id = event_instance_id;

DELETE FROM analysis_instances WHERE analysis_engine_id = 4;

END $$;
