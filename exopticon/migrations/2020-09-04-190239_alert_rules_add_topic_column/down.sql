-- This file should undo anything in `up.sql`

ALTER TABLE alert_rules DROP COLUMN notification_topic;

