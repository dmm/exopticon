// @generated automatically by Diesel CLI.

diesel::table! {
    camera_group_memberships (id) {
        id -> Uuid,
        camera_group_name -> Text,
        camera_name -> Text,
        display_order -> Int4,
    }
}

diesel::table! {
    camera_groups (name) {
        name -> Text,
        display_name -> Text,
    }
}

diesel::table! {
    cameras (name) {
        name -> Text,
        display_name -> Text,
        storage_group_name -> Text,
        ip -> Text,
        onvif_port -> Int4,
        mac -> Text,
        username -> Text,
        password -> Text,
        rtsp_url -> Text,
        ptz_type -> Text,
        ptz_profile_token -> Text,
        enabled -> Bool,
        ptz_x_step_size -> Int2,
        ptz_y_step_size -> Int2,
    }
}

diesel::table! {
    storage_groups (name) {
        name -> Text,
        display_name -> Text,
        storage_path -> Text,
        max_storage_size -> Int8,
    }
}

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        name -> Text,
        user_name -> Text,
        session_key -> Text,
        is_token -> Bool,
        expiration -> Timestamptz,
    }
}

diesel::table! {
    users (username) {
        username -> Text,
        display_name -> Text,
        password -> Text,
    }
}

diesel::table! {
    video_files (id) {
        id -> Uuid,
        filename -> Text,
        size -> Int4,
        video_unit_id -> Uuid,
    }
}

diesel::table! {
    video_units (id) {
        id -> Uuid,
        camera_name -> Text,
        begin_time -> Timestamptz,
        end_time -> Timestamptz,
    }
}

diesel::joinable!(camera_group_memberships -> camera_groups (camera_group_name));
diesel::joinable!(camera_group_memberships -> cameras (camera_name));
diesel::joinable!(cameras -> storage_groups (storage_group_name));
diesel::joinable!(user_sessions -> users (user_name));
diesel::joinable!(video_files -> video_units (video_unit_id));
diesel::joinable!(video_units -> cameras (camera_name));

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
