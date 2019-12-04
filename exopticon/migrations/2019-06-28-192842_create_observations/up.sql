CREATE TABLE observations (
       id BIGSERIAL PRIMARY KEY,
       video_unit_id INTEGER NOT NULL REFERENCES video_units,
       frame_offset INTEGER NOT NULL,
       tag TEXT NOT NULL,
       details TEXT NOT NULL,
       score SMALLINT NOT NULL CHECK (score > -1 AND score < 101),
       ul_x SMALLINT NOT NULL CHECK (ul_x > -2),
       ul_y SMALLINT NOT NULL CHECK (ul_y > -2),
       lr_x SMALLINT NOT NULL CHECK (lr_x > -2),
       lr_y SMALLINT NOT NULL CHECK (lr_y > -2),
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
)
