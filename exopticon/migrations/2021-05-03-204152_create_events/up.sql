-- Your SQL goes here

CREATE TABLE events (
       id uuid PRIMARY KEY,
       tag TEXT NOT NULL,
       camera_id INT NOT NULL REFERENCES cameras,
       begin_time TIMESTAMP WITH TIME ZONE NOT NULL,
       end_time TIMESTAMP WITH TIME ZONE NOT NULL,
       display_observation_id BIGINT NOT NULL REFERENCES observations
);

CREATE INDEX events_begin_time_idx ON events (begin_time);

CREATE TABLE event_observations (
       id BIGSERIAL PRIMARY KEY,
       event_id uuid NOT NULL REFERENCES events,
       observation_id BIGINT NOT NULL REFERENCES observations
);

CREATE INDEX event_observations_event_id_idx ON event_observations (event_id);

CREATE TABLE observation_snapshots (
       observation_id BIGINT PRIMARY KEY REFERENCES observations,
       snapshot_path TEXT NOT NULL,
       snapshot_size INT NOT NULL
);

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(4, 'event', 'v1.0', 'frigate/event.py' )
ON CONFLICT DO NOTHING;
