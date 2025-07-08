// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    camera_group_memberships (id) {
        id -> Binary,
        camera_group_id -> Binary,
        camera_id -> Binary,
        display_order -> Integer,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    camera_groups (id) {
        id -> Binary,
        name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    cameras (id) {
        id -> Binary,
        storage_group_id -> Binary,
        name -> Text,
        ip -> Text,
        onvif_port -> Integer,
        mac -> Text,
        username -> Text,
        password -> Text,
        rtsp_url -> Text,
        ptz_type -> Text,
        ptz_profile_token -> Text,
        enabled -> Integer,
        ptz_x_step_size -> Integer,
        ptz_y_step_size -> Integer,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    storage_groups (id) {
        id -> Binary,
        name -> Text,
        storage_path -> Text,
        max_storage_size -> Integer,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    user_sessions (id) {
        id -> Binary,
        name -> Text,
        user_id -> Binary,
        session_key -> Text,
        is_token -> Integer,
        expiration -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    users (id) {
        id -> Binary,
        username -> Text,
        password -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    video_files (id) {
        id -> Binary,
        video_unit_id -> Binary,
        filename -> Text,
        size -> Integer,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::db::uuid::*;

    video_units (id) {
        id -> Binary,
        camera_id -> Binary,
        begin_time -> Text,
        end_time -> Text,
    }
}

diesel::joinable!(camera_group_memberships -> camera_groups (camera_group_id));
diesel::joinable!(camera_group_memberships -> cameras (camera_id));
diesel::joinable!(cameras -> storage_groups (storage_group_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(video_files -> video_units (video_unit_id));
diesel::joinable!(video_units -> cameras (camera_id));

diesel::allow_tables_to_appear_in_same_query!(
    camera_group_memberships,
    camera_groups,
    cameras,
    storage_groups,
    user_sessions,
    users,
    video_files,
    video_units,
);
