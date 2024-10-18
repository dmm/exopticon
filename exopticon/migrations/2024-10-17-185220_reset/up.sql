-- Your SQL goes here
CREATE TABLE "camera_groups"(
	"id" UUID NOT NULL PRIMARY KEY,
	"name" TEXT NOT NULL
);

CREATE TABLE "users"(
	"id" UUID NOT NULL PRIMARY KEY,
	"username" TEXT NOT NULL,
	"password" TEXT NOT NULL
);

CREATE TABLE "storage_groups"(
	"id" UUID NOT NULL PRIMARY KEY,
	"name" TEXT NOT NULL,
	"storage_path" TEXT NOT NULL,
	"max_storage_size" BIGINT NOT NULL
);

CREATE TABLE "cameras"(
	"id" UUID NOT NULL PRIMARY KEY,
	"storage_group_id" UUID NOT NULL,
	"name" TEXT NOT NULL,
	"ip" TEXT NOT NULL,
	"onvif_port" INTEGER NOT NULL,
	"mac" TEXT NOT NULL,
	"username" TEXT NOT NULL,
	"password" TEXT NOT NULL,
	"rtsp_url" TEXT NOT NULL,
	"ptz_type" TEXT NOT NULL,
	"ptz_profile_token" TEXT NOT NULL,
	"enabled" BOOL NOT NULL,
	"ptz_x_step_size" SMALLINT NOT NULL,
	"ptz_y_step_size" SMALLINT NOT NULL,
	FOREIGN KEY ("storage_group_id") REFERENCES "storage_groups"("id")
);

CREATE TABLE "user_sessions"(
	"id" UUID NOT NULL PRIMARY KEY,
	"name" TEXT NOT NULL,
	"user_id" UUID NOT NULL,
	"session_key" TEXT NOT NULL,
	"is_token" BOOL NOT NULL,
	"expiration" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("user_id") REFERENCES "users"("id")
);

CREATE TABLE "video_units"(
	"id" UUID NOT NULL PRIMARY KEY,
	"camera_id" UUID NOT NULL,
	"begin_time" TIMESTAMPTZ NOT NULL,
	"end_time" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("camera_id") REFERENCES "cameras"("id")
);

CREATE TABLE "video_files"(
	"id" UUID NOT NULL PRIMARY KEY,
	"filename" TEXT NOT NULL,
	"size" INTEGER NOT NULL,
	"video_unit_id" UUID NOT NULL,
	FOREIGN KEY ("video_unit_id") REFERENCES "video_units"("id")
);

CREATE TABLE "camera_group_memberships"(
	"id" UUID NOT NULL PRIMARY KEY,
	"camera_group_id" UUID NOT NULL,
	"camera_id" UUID NOT NULL,
	"display_order" INTEGER NOT NULL,
	FOREIGN KEY ("camera_group_id") REFERENCES "camera_groups"("id"),
	FOREIGN KEY ("camera_id") REFERENCES "cameras"("id")
);

