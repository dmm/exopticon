-- Your SQL goes here

CREATE TABLE events (
       id uuid PRIMARY KEY,
       tag TEXT NOT NULL,
       inserted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE event_observations (
       id BIGSERIAL PRIMARY KEY,
       event_id uuid NOT NULL REFERENCES events,
       observation_id BIGINT NOT NULL REFERENCES observations
);

CREATE INDEX event_observations_event_id_idx ON event_observations (event_id);

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(4, 'event', 'v1.0', 'frigate/event.py' );

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON events
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
