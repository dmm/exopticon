-- Recreate UUID identities for configured resources. The generated UUIDs are
-- new values; the forward migration intentionally makes names the durable IDs.

ALTER TABLE storage_groups ADD COLUMN id UUID DEFAULT gen_random_uuid();
ALTER TABLE cameras ADD COLUMN id UUID DEFAULT gen_random_uuid();
ALTER TABLE camera_groups ADD COLUMN id UUID DEFAULT gen_random_uuid();
ALTER TABLE users ADD COLUMN id UUID DEFAULT gen_random_uuid();

ALTER TABLE storage_groups ALTER COLUMN id SET NOT NULL;
ALTER TABLE cameras ALTER COLUMN id SET NOT NULL;
ALTER TABLE camera_groups ALTER COLUMN id SET NOT NULL;
ALTER TABLE users ALTER COLUMN id SET NOT NULL;

ALTER TABLE cameras ADD COLUMN storage_group_id UUID;
UPDATE cameras
SET storage_group_id = storage_groups.id
FROM storage_groups
WHERE cameras.storage_group_name = storage_groups.name;
ALTER TABLE cameras ALTER COLUMN storage_group_id SET NOT NULL;

ALTER TABLE camera_group_memberships ADD COLUMN camera_group_id UUID;
ALTER TABLE camera_group_memberships ADD COLUMN camera_id UUID;
UPDATE camera_group_memberships
SET camera_group_id = camera_groups.id
FROM camera_groups
WHERE camera_group_memberships.camera_group_name = camera_groups.name;
UPDATE camera_group_memberships
SET camera_id = cameras.id
FROM cameras
WHERE camera_group_memberships.camera_name = cameras.name;
ALTER TABLE camera_group_memberships ALTER COLUMN camera_group_id SET NOT NULL;
ALTER TABLE camera_group_memberships ALTER COLUMN camera_id SET NOT NULL;

ALTER TABLE video_units ADD COLUMN camera_id UUID;
UPDATE video_units
SET camera_id = cameras.id
FROM cameras
WHERE video_units.camera_name = cameras.name;
ALTER TABLE video_units ALTER COLUMN camera_id SET NOT NULL;

ALTER TABLE user_sessions ADD COLUMN user_id UUID;
UPDATE user_sessions
SET user_id = users.id
FROM users
WHERE user_sessions.user_name = users.username;
ALTER TABLE user_sessions ALTER COLUMN user_id SET NOT NULL;

ALTER TABLE cameras DROP CONSTRAINT cameras_storage_group_name_fkey;
ALTER TABLE camera_group_memberships DROP CONSTRAINT camera_group_memberships_camera_group_name_fkey;
ALTER TABLE camera_group_memberships DROP CONSTRAINT camera_group_memberships_camera_name_fkey;
ALTER TABLE video_units DROP CONSTRAINT video_units_camera_name_fkey;
ALTER TABLE user_sessions DROP CONSTRAINT user_sessions_user_name_fkey;

ALTER TABLE storage_groups DROP CONSTRAINT storage_groups_pkey;
ALTER TABLE cameras DROP CONSTRAINT cameras_pkey;
ALTER TABLE camera_groups DROP CONSTRAINT camera_groups_pkey;
ALTER TABLE users DROP CONSTRAINT users_pkey;

UPDATE camera_groups SET name = 'ALL' WHERE name = 'all';

ALTER TABLE camera_group_memberships DROP COLUMN camera_group_name;
ALTER TABLE camera_group_memberships DROP COLUMN camera_name;
ALTER TABLE cameras DROP COLUMN storage_group_name;
ALTER TABLE video_units DROP COLUMN camera_name;
ALTER TABLE user_sessions DROP COLUMN user_name;

ALTER TABLE storage_groups DROP COLUMN display_name;
ALTER TABLE cameras DROP COLUMN display_name;
ALTER TABLE camera_groups DROP COLUMN display_name;
ALTER TABLE users DROP COLUMN display_name;

ALTER TABLE storage_groups ADD PRIMARY KEY (id);
ALTER TABLE cameras ADD PRIMARY KEY (id);
ALTER TABLE camera_groups ADD PRIMARY KEY (id);
ALTER TABLE users ADD PRIMARY KEY (id);

ALTER TABLE cameras
ADD CONSTRAINT cameras_storage_group_id_fkey
FOREIGN KEY (storage_group_id) REFERENCES storage_groups(id);

ALTER TABLE camera_group_memberships
ADD CONSTRAINT camera_group_memberships_camera_group_id_fkey
FOREIGN KEY (camera_group_id) REFERENCES camera_groups(id);

ALTER TABLE camera_group_memberships
ADD CONSTRAINT camera_group_memberships_camera_id_fkey
FOREIGN KEY (camera_id) REFERENCES cameras(id);

ALTER TABLE user_sessions
ADD CONSTRAINT user_sessions_user_id_fkey
FOREIGN KEY (user_id) REFERENCES users(id);

ALTER TABLE video_units
ADD CONSTRAINT video_units_camera_id_fkey
FOREIGN KEY (camera_id) REFERENCES cameras(id);
