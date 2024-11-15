-- This file should undo anything in `up.sql`

DELETE FROM camera_group_memberships WHERE camera_group_id = '34e79812-df14-4773-a9f4-f766c799aa62';

DELETE FROM camera_groups WHERE id = '34e79812-df14-4773-a9f4-f766c799aa62'
;
