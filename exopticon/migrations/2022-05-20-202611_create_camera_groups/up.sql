-- Your SQL goes here

CREATE TABLE camera_groups (
       id SERIAL PRIMARY KEY,
       name TEXT NOT NULL
);

CREATE TABLE camera_group_memberships (
       id SERIAL PRIMARY KEY,
       camera_group_id INT NOT NULL REFERENCES camera_groups,
       camera_id INT NOT NULL REFERENCES cameras,
       display_order INT NOT NULL,
       UNIQUE (camera_group_id, camera_id)
);
