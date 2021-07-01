-- Your SQL goes here

CREATE INDEX observations_video_unit_id_idx ON observations (video_unit_id);

CREATE INDEX video_files_video_unit_id_idx ON video_files (video_unit_id);
