ALTER TABLE cameras ADD COLUMN ip TEXT NOT NULL DEFAULT '';
ALTER TABLE cameras ADD COLUMN onvif_port INTEGER NOT NULL DEFAULT 80;
ALTER TABLE cameras ADD COLUMN mac TEXT NOT NULL DEFAULT '';
ALTER TABLE cameras ADD COLUMN ptz_type TEXT NOT NULL DEFAULT 'none';

UPDATE cameras
SET ptz_type = CASE WHEN ptz_enabled THEN 'onvif_relative' ELSE 'none' END,
    onvif_profile_token = CASE WHEN onvif_device_url IS NULL THEN ptz_profile_token ELSE onvif_profile_token END;

-- The legacy fields can represent the host and port of a PTZ endpoint.
WITH endpoints AS (
    SELECT name,
        CASE WHEN ptz_device_url LIKE 'https://%' THEN 443 ELSE 80 END AS default_port,
        substr(ptz_device_url, instr(ptz_device_url, '://') + 3) AS address
    FROM cameras WHERE ptz_device_url IS NOT NULL
), authorities AS (
    SELECT name, default_port, substr(address, 1, instr(address || '/', '/') - 1) AS authority
    FROM endpoints
), hosts AS (
    SELECT name, default_port, authority,
        CASE
            WHEN substr(authority, 1, 1) = '[' THEN substr(authority, 1, instr(authority, ']'))
            ELSE substr(authority, 1, instr(authority || ':', ':') - 1)
        END AS host
    FROM authorities
)
UPDATE cameras
SET ip = hosts.host,
    onvif_port = COALESCE(NULLIF(substr(hosts.authority, length(hosts.host) + 2), ''), hosts.default_port)
FROM hosts
WHERE cameras.name = hosts.name;

ALTER TABLE cameras DROP COLUMN ptz_enabled;
ALTER TABLE cameras DROP COLUMN ptz_device_url;
ALTER TABLE cameras DROP COLUMN ptz_profile_token;
