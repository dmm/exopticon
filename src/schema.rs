table! {
    camera_groups (id) {
        id -> Int4,
        name -> Varchar,
        storage_path -> Varchar,
        max_storage_size -> Int8,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    cameras (id) {
        id -> Int4,
        camera_group_id -> Int4,
        name -> Varchar,
        ip -> Varchar,
        onvif_port -> Int4,
        mac -> Varchar,
        username -> Varchar,
        password -> Varchar,
        rtsp_url -> Varchar,
        ptz_type -> Varchar,
        ptz_profile_token -> Varchar,
        enabled -> Bool,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    video_files (id) {
        id -> Int4,
        video_unit_id -> Int4,
        filename -> Varchar,
        size -> Int4,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    video_units (id) {
        id -> Int4,
        camera_id -> Int4,
        monotonic_index -> Int4,
        begin_time -> Timestamp,
        end_time -> Timestamp,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(cameras -> camera_groups (camera_group_id));
joinable!(video_files -> video_units (video_unit_id));
joinable!(video_units -> cameras (camera_id));

allow_tables_to_appear_in_same_query!(camera_groups, cameras, video_files, video_units,);
