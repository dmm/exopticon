-- AUTOINCREMENT preserves the high-water mark after retention deletes the newest file.
CREATE TABLE video_files_new (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    video_unit_id INTEGER NOT NULL,
    FOREIGN KEY (video_unit_id) REFERENCES video_units(id)
        ON DELETE CASCADE
);
INSERT INTO video_files_new (id, filename, size, video_unit_id)
SELECT id, filename, size, video_unit_id FROM video_files;
DROP TABLE video_files;
ALTER TABLE video_files_new RENAME TO video_files;
CREATE INDEX video_files_video_unit_idx ON video_files(video_unit_id);
