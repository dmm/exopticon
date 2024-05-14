-- This file should undo anything in `up.sql`

ALTER TABLE cameras DROP COLUMN ptz_x_step_size;
ALTER TABLE cameras DROP COLUMN ptz_y_step_size;

