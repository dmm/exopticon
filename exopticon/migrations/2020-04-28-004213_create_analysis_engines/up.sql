-- Your SQL goes here

CREATE TABLE analysis_engines (
       id SERIAL PRIMARY KEY,
       name TEXT NOT NULL,
       version TEXT NOT NULL,
       entry_point TEXT NOT NULL,
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE analysis_instances (
       id SERIAL PRIMARY KEY,
       analysis_engine_id INTEGER NOT NULL REFERENCES analysis_engines,
       name TEXT NOT NULL,
       max_fps INTEGER NOT NULL DEFAULT 0,
       enabled BOOLEAN NOT NULL DEFAULT 't',
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE analysis_subscriptions (
       id SERIAL PRIMARY KEY,
       analysis_instance_id INTEGER NOT NULL REFERENCES analysis_instances,
       camera_id INTEGER REFERENCES cameras,
       source_analysis_instance_id INTEGER REFERENCES analysis_instances,
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
       CHECK ((camera_id IS NOT NULL AND source_analysis_instance_id IS NULL)
              OR (source_analysis_instance_id IS NOT NULL AND camera_id IS NULL))
);

CREATE TABLE subscription_masks (
       id SERIAL PRIMARY KEY,
       analysis_subscription_id INTEGER NOT NULL REFERENCES analysis_subscriptions,
       ul_x SMALLINT NOT NULL CHECK (ul_x > -2),
       ul_y SMALLINT NOT NULL CHECK (ul_y > -2),
       lr_x SMALLINT NOT NULL CHECK (lr_x > -2),
       lr_y SMALLINT NOT NULL CHECK (lr_y > -2),
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP

);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON analysis_engines
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON analysis_instances
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON analysis_subscriptions
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON subscription_masks
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

