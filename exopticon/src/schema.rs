// @generated automatically by Diesel CLI.

diesel::table! {
    camera_group_memberships (id) {
        id -> BigInt,
        camera_group_name -> Text,
        camera_name -> Text,
        display_order -> Integer,
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
        onvif_port -> Integer,
        mac -> Text,
        username -> Text,
        password -> Text,
        rtsp_url -> Text,
        ptz_type -> Text,
        ptz_profile_token -> Text,
        enabled -> Bool,
        ptz_x_step_size -> SmallInt,
        ptz_y_step_size -> SmallInt,
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
        id -> BigInt,
        name -> Text,
        user_name -> Text,
        session_key -> Text,
        is_token -> Bool,
        expiration_us -> BigInt,
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
        id -> BigInt,
        filename -> Text,
        size -> Integer,
        video_unit_id -> BigInt,
    }
}

diesel::table! {
    video_units (id) {
        id -> BigInt,
        camera_name -> Text,
        begin_time_us -> BigInt,
        end_time_us -> BigInt,
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
