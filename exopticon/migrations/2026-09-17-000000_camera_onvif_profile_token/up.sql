ALTER TABLE cameras ADD COLUMN onvif_profile_token TEXT;

UPDATE cameras
SET onvif_profile_token = NULLIF(TRIM(ptz_profile_token), '');

ALTER TABLE cameras DROP COLUMN ptz_profile_token;
