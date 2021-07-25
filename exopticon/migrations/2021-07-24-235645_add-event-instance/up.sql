-- Your SQL goes here

DO $$
DECLARE event_instance_id Int;
BEGIN

INSERT INTO analysis_instances
(analysis_engine_id, name, max_fps, enabled)
VALUES
(4, 'Event Detection', 0, 't')
RETURNING id INTO event_instance_id
;

-- Insert subscription to Yolo worker
INSERT INTO analysis_subscriptions
(analysis_instance_id, source_analysis_instance_id)
(SELECT event_instance_id, id FROM analysis_instances WHERE analysis_engine_id = 2)
;

-- Insert subscription to Coral worker
INSERT INTO analysis_subscriptions
(analysis_instance_id, source_analysis_instance_id)
(SELECT event_instance_id, id FROM analysis_instances WHERE analysis_engine_id = 3)
;

END $$;
