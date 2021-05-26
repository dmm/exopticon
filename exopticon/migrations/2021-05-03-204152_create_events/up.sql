-- Your SQL goes here

CREATE TABLE events (
       id BIGSERIAL PRIMARY KEY,
       tag TEXT NOT NULL
);

CREATE TABLE event_observations (
       id BIGSERIAL PRIMARY KEY,
       event_id BIGINT NOT NULL REFERENCES events,
       observation_id BIGINT NOT NULL REFERENCES observations
);

INSERT INTO analysis_engines
( id, name, version, entry_point )
VALUES
(4, 'event', 'v1.0', 'frigate/event.py' );
