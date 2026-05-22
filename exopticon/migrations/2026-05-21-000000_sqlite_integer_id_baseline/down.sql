DROP INDEX user_sessions_user_token_idx;
DROP INDEX user_sessions_session_key_idx;
DROP INDEX cameras_storage_group_idx;
DROP INDEX video_files_video_unit_idx;
DROP INDEX video_units_camera_time_idx;

DROP TABLE video_files;
DROP TABLE video_units;
DROP TABLE user_sessions;
DROP TABLE users;
DROP TABLE camera_group_memberships;
DROP TABLE camera_groups;
DROP TABLE cameras;
DROP TABLE storage_groups;
