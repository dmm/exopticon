CREATE TABLE users (
       id SERIAL PRIMARY KEY,
       username VARCHAR(100) NOT NULL,
       password VARCHAR(64) NOT NULL,
       timezone VARCHAR(100) NOT NULL,
       inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp,
       updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT timezone('UTC', NOW())::timestamp
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
