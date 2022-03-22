-- Your SQL goes here

CREATE TABLE user_sessions (
       id SERIAL PRIMARY KEY,
       name TEXT NOT NULL DEFAULT '',
       user_id INT NOT NULL REFERENCES users,
       session_key TEXT NOT NULL UNIQUE,
       is_token BOOLEAN NOT NULL DEFAULT 'f',
       expiration TIMESTAMPTZ NOT NULL,
       inserted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
