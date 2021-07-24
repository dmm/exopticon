-- Your SQL goes here

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(1, 'motion', 'v1.0', 'frigate/motion.py' )
ON CONFLICT DO NOTHING;

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(2, 'yolov4', 'v1.0', 'yolov4/darknet.py' )
ON CONFLICT DO NOTHING;

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(3, 'coral', 'v1.0', 'coral/coral.py' )
ON CONFLICT DO NOTHING;

INSERT INTO analysis_instances
(analysis_engine_id, name, max_fps, enabled)
VALUES
(2, 'Yolov4 Object Detection', 0, 't')
;

INSERT INTO analysis_instances
(analysis_engine_id, name, max_fps, enabled)
VALUES
(3, 'Coral Object Detection', 0, 't')
;
