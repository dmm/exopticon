-- Your SQL goes here
CREATE TABLE `cameras`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`storage_group_id` BLOB NOT NULL,
	`name` TEXT NOT NULL,
	`ip` TEXT NOT NULL,
	`onvif_port` INTEGER NOT NULL,
	`mac` TEXT NOT NULL,
	`username` TEXT NOT NULL,
	`password` TEXT NOT NULL,
	`rtsp_url` TEXT NOT NULL,
	`ptz_type` TEXT NOT NULL,
	`ptz_profile_token` TEXT NOT NULL,
	`enabled` INTEGER NOT NULL DEFAULT 0,
	`ptz_x_step_size` INTEGER NOT NULL,
	`ptz_y_step_size` INTEGER NOT NULL,
	FOREIGN KEY (`storage_group_id`) REFERENCES `storage_groups`(`id`)
) STRICT, WITHOUT ROWID;

CREATE TABLE `user_sessions`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL,
	`user_id` BLOB NOT NULL,
	`session_key` TEXT NOT NULL,
	`is_token` INTEGER NOT NULL,
	`expiration` TEXT NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`)
) STRICT, WITHOUT ROWID;

CREATE TABLE `video_units`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`camera_id` BLOB NOT NULL,
	`begin_time` TEXT NOT NULL,
	`end_time` TEXT NOT NULL,
	FOREIGN KEY (`camera_id`) REFERENCES `cameras`(`id`)
) STRICT, WITHOUT ROWID;

CREATE TABLE `video_files`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`video_unit_id` BLOB NOT NULL,
	`filename` TEXT NOT NULL,
	`size` INTEGER NOT NULL,
	FOREIGN KEY (`video_unit_id`) REFERENCES `video_units`(`id`)
) STRICT, WITHOUT ROWID;

CREATE TABLE `camera_group_memberships`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`camera_group_id` BLOB NOT NULL,
	`camera_id` BLOB NOT NULL,
	`display_order` INTEGER NOT NULL,
	FOREIGN KEY (`camera_group_id`) REFERENCES `camera_groups`(`id`),
	FOREIGN KEY (`camera_id`) REFERENCES `cameras`(`id`)
) STRICT, WITHOUT ROWID;

CREATE TABLE `storage_groups`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL,
	`storage_path` TEXT NOT NULL,
	`max_storage_size` INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE `camera_groups`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TABLE `users`(
	`id` BLOB NOT NULL PRIMARY KEY,
	`username` TEXT NOT NULL,
	`password` TEXT NOT NULL
) STRICT, WITHOUT ROWID;

