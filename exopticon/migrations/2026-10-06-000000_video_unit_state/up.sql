-- Adding the column in place preserves video_files and its ID high-water mark.
-- The default permits ADD COLUMN on populated tables; the backfill below
-- validates every legacy row instead of accepting that default as its state.
ALTER TABLE video_units ADD COLUMN state TEXT NOT NULL DEFAULT 'Reserved'
    CHECK (state IN ('Reserved', 'Open', 'Finalized'));

-- Capture creates exactly one file per unit. Missing or multiple files, invalid
-- intervals, and unsupported size sentinels cannot be classified safely.
-- Assigning NULL makes such rows fail the NOT NULL constraint, rolling back
-- the migration in Diesel's migration transaction for manual cleanup/retry.
UPDATE video_units
SET state = CASE
    WHEN (SELECT COUNT(*) FROM video_files WHERE video_unit_id = video_units.id) = 1
    THEN (
        SELECT CASE
            WHEN video_units.begin_time_us = video_units.end_time_us AND size = 0
                THEN 'Reserved'
            WHEN video_units.begin_time_us = video_units.end_time_us AND size = -1
                THEN 'Open'
            -- A closed file can be empty, or have size -1 when its byte length
            -- exceeds i32. Preserve those existing file-size semantics.
            WHEN video_units.begin_time_us < video_units.end_time_us AND size >= -1
                THEN 'Finalized'
            ELSE NULL
        END
        FROM video_files
        WHERE video_unit_id = video_units.id
    )
    ELSE NULL
END;
