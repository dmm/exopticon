-- Your SQL goes here

INSERT INTO camera_groups (id, name)
VALUES ('34e79812-df14-4773-a9f4-f766c799aa62', 'ALL')
;

INSERT INTO camera_group_memberships (id, camera_group_id, camera_id, display_order)
SELECT gen_random_uuid(), '34e79812-df14-4773-a9f4-f766c799aa62', id, ROW_NUMBER() OVER (ORDER BY id)
FROM cameras;
