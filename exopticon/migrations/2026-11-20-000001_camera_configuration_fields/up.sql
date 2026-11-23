ALTER TABLE cameras ADD COLUMN ptz_enabled BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE cameras ADD COLUMN ptz_device_url TEXT;
ALTER TABLE cameras ADD COLUMN ptz_profile_token TEXT;

UPDATE cameras
SET ptz_enabled = ptz_type <> 'none',
    ptz_device_url = CASE
        WHEN onvif_device_url IS NULL AND ptz_type <> 'none'
            AND trim(ip) <> '' AND onvif_port BETWEEN 1 AND 65535
        THEN 'http://' ||
            CASE WHEN instr(ip, ':') > 0 AND substr(ip, 1, 1) <> '['
                THEN '[' || trim(ip) || ']' ELSE trim(ip) END ||
            ':' || onvif_port || '/onvif/device_service'
    END,
    ptz_profile_token = CASE WHEN onvif_device_url IS NULL THEN onvif_profile_token END,
    onvif_profile_token = CASE WHEN onvif_device_url IS NOT NULL THEN onvif_profile_token END;

ALTER TABLE cameras DROP COLUMN ptz_type;
ALTER TABLE cameras DROP COLUMN ip;
ALTER TABLE cameras DROP COLUMN onvif_port;
ALTER TABLE cameras DROP COLUMN mac;
