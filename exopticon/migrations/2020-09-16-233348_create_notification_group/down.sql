-- This file should undo anything in `up.sql`

DROP TABLE notification_contacts;

ALTER TABLE alert_rules
RENAME COLUMN contact_group TO notification_topic;
