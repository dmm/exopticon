-- Your SQL goes here

CREATE TABLE camera_groups (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    storage_path VARCHAR NOT NULL,
    max_storage_size integer NOT NULL,
    inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp
);

CREATE TABLE cameras (
    id SERIAL PRIMARY KEY,
    camera_group_id INTEGER NOT NULL REFERENCES camera_groups,
    name VARCHAR NOT NULL,
    ip VARCHAR NOT NULL,
    onvif_port INTEGER NOT NULL,
    mac VARCHAR NOT NULL,
    username VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    rtsp_url VARCHAR NOT NULL,
    ptz_type VARCHAR NOT NULL,
    ptz_profile_token VARCHAR NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 't',
    inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp
);

CREATE TABLE video_units (
    id SERIAL PRIMARY KEY,
    camera_id INTEGER NOT NULL REFERENCES cameras,
    monotonic_index INTEGER NOT NULL,
    begin_time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end_time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp
);

CREATE TABLE video_files (
    id SERIAL PRIMARY KEY,
    video_unit_id INTEGER NOT NULL REFERENCES video_units,
    filename VARCHAR NOT NULL,
    size INTEGER NOT NULL,
    inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp
);


-- Configure update triggers
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = timezone('UTC', NOW())::timestamp;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON camera_groups
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON cameras
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON video_units
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON video_files
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
