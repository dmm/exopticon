DROP INDEX video_files_video_unit_idx;
CREATE TABLE video_files_old (
    id INTEGER NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    video_unit_id INTEGER NOT NULL,
    FOREIGN KEY (video_unit_id) REFERENCES video_units(id)
        ON DELETE CASCADE
);
INSERT INTO video_files_old (id, filename, size, video_unit_id)
SELECT id, filename, size, video_unit_id FROM video_files;
DROP TABLE video_files;
ALTER TABLE video_files_old RENAME TO video_files;
CREATE INDEX video_files_video_unit_idx ON video_files(video_unit_id);
