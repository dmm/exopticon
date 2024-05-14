-- Your SQL goes here

ALTER TABLE cameras ADD COLUMN ptz_x_step_size smallint
      NOT NULL
      DEFAULT 10
      CHECK (ptz_x_step_size <= 100)
      CHECK (ptz_x_step_size >= -100);
ALTER TABLE cameras ADD COLUMN ptz_y_step_size smallint
      NOT NULL
      DEFAULT 10
      CHECK (ptz_y_step_size <= 100)
      CHECK (ptz_y_step_size >= -100);
