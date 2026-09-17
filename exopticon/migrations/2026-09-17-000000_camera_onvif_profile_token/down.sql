ALTER TABLE cameras ADD COLUMN ptz_profile_token TEXT NOT NULL DEFAULT '';

UPDATE cameras
SET ptz_profile_token = COALESCE(onvif_profile_token, '');

ALTER TABLE cameras DROP COLUMN onvif_profile_token;
