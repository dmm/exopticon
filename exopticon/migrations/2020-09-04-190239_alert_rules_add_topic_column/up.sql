-- Your SQL goes here

ALTER TABLE alert_rules ADD COLUMN notification_topic VARCHAR;

UPDATE alert_rules SET notification_topic = '/home/exopticon/alert';

ALTER TABLE alert_rules ALTER COLUMN notification_topic SET NOT NULL;
