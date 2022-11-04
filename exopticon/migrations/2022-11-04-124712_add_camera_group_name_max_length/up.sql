-- Your SQL goes here
ALTER TABLE camera_groups
ADD CONSTRAINT max_name_length
CHECK (char_length(name) <= 15);
