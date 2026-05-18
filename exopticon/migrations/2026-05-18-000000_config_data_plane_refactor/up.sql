-- Convert configured resources from UUID identities to stable config names.
-- Runtime/data-plane records keep UUID primary keys.

ALTER TABLE storage_groups ADD COLUMN display_name TEXT;
ALTER TABLE cameras ADD COLUMN display_name TEXT;
ALTER TABLE camera_groups ADD COLUMN display_name TEXT;
ALTER TABLE users ADD COLUMN display_name TEXT;

UPDATE storage_groups SET display_name = name;
UPDATE cameras SET display_name = name;
UPDATE camera_groups SET name = 'all' WHERE name = 'ALL';
UPDATE camera_groups SET display_name = name;
UPDATE users SET display_name = username;

ALTER TABLE storage_groups ALTER COLUMN display_name SET NOT NULL;
ALTER TABLE cameras ALTER COLUMN display_name SET NOT NULL;
ALTER TABLE camera_groups ALTER COLUMN display_name SET NOT NULL;
ALTER TABLE users ALTER COLUMN display_name SET NOT NULL;

ALTER TABLE cameras ADD COLUMN storage_group_name TEXT;
UPDATE cameras
SET storage_group_name = storage_groups.name
FROM storage_groups
WHERE cameras.storage_group_id = storage_groups.id;
ALTER TABLE cameras ALTER COLUMN storage_group_name SET NOT NULL;

ALTER TABLE camera_group_memberships ADD COLUMN camera_group_name TEXT;
ALTER TABLE camera_group_memberships ADD COLUMN camera_name TEXT;
UPDATE camera_group_memberships
SET camera_group_name = camera_groups.name
FROM camera_groups
WHERE camera_group_memberships.camera_group_id = camera_groups.id;
UPDATE camera_group_memberships
SET camera_name = cameras.name
FROM cameras
WHERE camera_group_memberships.camera_id = cameras.id;
ALTER TABLE camera_group_memberships ALTER COLUMN camera_group_name SET NOT NULL;
ALTER TABLE camera_group_memberships ALTER COLUMN camera_name SET NOT NULL;

ALTER TABLE video_units ADD COLUMN camera_name TEXT;
UPDATE video_units
SET camera_name = cameras.name
FROM cameras
WHERE video_units.camera_id = cameras.id;
ALTER TABLE video_units ALTER COLUMN camera_name SET NOT NULL;

ALTER TABLE user_sessions ADD COLUMN user_name TEXT;
UPDATE user_sessions
SET user_name = users.username
FROM users
WHERE user_sessions.user_id = users.id;
ALTER TABLE user_sessions ALTER COLUMN user_name SET NOT NULL;

ALTER TABLE camera_group_memberships DROP CONSTRAINT camera_group_memberships_camera_group_id_fkey;
ALTER TABLE camera_group_memberships DROP CONSTRAINT camera_group_memberships_camera_id_fkey;
ALTER TABLE cameras DROP CONSTRAINT cameras_storage_group_id_fkey;
ALTER TABLE user_sessions DROP CONSTRAINT user_sessions_user_id_fkey;
ALTER TABLE video_units DROP CONSTRAINT video_units_camera_id_fkey;

ALTER TABLE storage_groups DROP CONSTRAINT storage_groups_pkey;
ALTER TABLE cameras DROP CONSTRAINT cameras_pkey;
ALTER TABLE camera_groups DROP CONSTRAINT camera_groups_pkey;
ALTER TABLE users DROP CONSTRAINT users_pkey;

ALTER TABLE camera_group_memberships DROP COLUMN camera_group_id;
ALTER TABLE camera_group_memberships DROP COLUMN camera_id;
ALTER TABLE cameras DROP COLUMN storage_group_id;
ALTER TABLE video_units DROP COLUMN camera_id;
ALTER TABLE user_sessions DROP COLUMN user_id;

ALTER TABLE storage_groups DROP COLUMN id;
ALTER TABLE cameras DROP COLUMN id;
ALTER TABLE camera_groups DROP COLUMN id;
ALTER TABLE users DROP COLUMN id;

ALTER TABLE storage_groups ADD PRIMARY KEY (name);
ALTER TABLE cameras ADD PRIMARY KEY (name);
ALTER TABLE camera_groups ADD PRIMARY KEY (name);
ALTER TABLE users ADD PRIMARY KEY (username);

ALTER TABLE cameras
ADD CONSTRAINT cameras_storage_group_name_fkey
FOREIGN KEY (storage_group_name) REFERENCES storage_groups(name);

ALTER TABLE camera_group_memberships
ADD CONSTRAINT camera_group_memberships_camera_group_name_fkey
FOREIGN KEY (camera_group_name) REFERENCES camera_groups(name);

ALTER TABLE camera_group_memberships
ADD CONSTRAINT camera_group_memberships_camera_name_fkey
FOREIGN KEY (camera_name) REFERENCES cameras(name);

ALTER TABLE video_units
ADD CONSTRAINT video_units_camera_name_fkey
FOREIGN KEY (camera_name) REFERENCES cameras(name);

ALTER TABLE user_sessions
ADD CONSTRAINT user_sessions_user_name_fkey
FOREIGN KEY (user_name) REFERENCES users(username);
