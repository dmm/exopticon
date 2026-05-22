CREATE TABLE storage_groups (
    name TEXT NOT NULL PRIMARY KEY,
    display_name TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    max_storage_size BIGINT NOT NULL
);

CREATE TABLE cameras (
    name TEXT NOT NULL PRIMARY KEY,
    display_name TEXT NOT NULL,
    storage_group_name TEXT NOT NULL,
    ip TEXT NOT NULL,
    onvif_port INTEGER NOT NULL,
    mac TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    rtsp_url TEXT NOT NULL,
    ptz_type TEXT NOT NULL,
    ptz_profile_token TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    ptz_x_step_size SMALLINT NOT NULL,
    ptz_y_step_size SMALLINT NOT NULL,
    FOREIGN KEY (storage_group_name) REFERENCES storage_groups(name)
        ON UPDATE CASCADE
);

CREATE TABLE camera_groups (
    name TEXT NOT NULL PRIMARY KEY,
    display_name TEXT NOT NULL
);

CREATE TABLE camera_group_memberships (
    id INTEGER NOT NULL PRIMARY KEY,
    camera_group_name TEXT NOT NULL,
    camera_name TEXT NOT NULL,
    display_order INTEGER NOT NULL,
    FOREIGN KEY (camera_group_name) REFERENCES camera_groups(name)
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY (camera_name) REFERENCES cameras(name)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

CREATE TABLE users (
    username TEXT NOT NULL PRIMARY KEY,
    display_name TEXT NOT NULL,
    password TEXT NOT NULL
);

CREATE TABLE user_sessions (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    user_name TEXT NOT NULL,
    session_key TEXT NOT NULL,
    is_token BOOLEAN NOT NULL,
    expiration_us BIGINT NOT NULL,
    FOREIGN KEY (user_name) REFERENCES users(username)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

CREATE TABLE video_units (
    id INTEGER NOT NULL PRIMARY KEY,
    camera_name TEXT NOT NULL,
    begin_time_us BIGINT NOT NULL,
    end_time_us BIGINT NOT NULL,
    FOREIGN KEY (camera_name) REFERENCES cameras(name)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

CREATE TABLE video_files (
    id INTEGER NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    video_unit_id INTEGER NOT NULL,
    FOREIGN KEY (video_unit_id) REFERENCES video_units(id)
        ON DELETE CASCADE
);

CREATE INDEX video_units_camera_time_idx
    ON video_units(camera_name, begin_time_us, end_time_us);

CREATE INDEX video_files_video_unit_idx
    ON video_files(video_unit_id);

CREATE INDEX cameras_storage_group_idx
    ON cameras(storage_group_name);

CREATE INDEX user_sessions_session_key_idx
    ON user_sessions(session_key, expiration_us);

CREATE INDEX user_sessions_user_token_idx
    ON user_sessions(user_name, is_token);
