table! {
    analysis_engines (id) {
        id -> Int4,
        name -> Text,
        version -> Text,
        entry_point -> Text,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    analysis_instances (id) {
        id -> Int4,
        analysis_engine_id -> Int4,
        name -> Text,
        max_fps -> Int4,
        enabled -> Bool,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    analysis_subscriptions (id) {
        id -> Int4,
        analysis_instance_id -> Int4,
        camera_id -> Nullable<Int4>,
        source_analysis_instance_id -> Nullable<Int4>,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

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
    observations (id) {
        id -> Int8,
        video_unit_id -> Int4,
        frame_offset -> Int4,
        tag -> Text,
        details -> Text,
        score -> Int2,
        ul_x -> Int2,
        ul_y -> Int2,
        lr_x -> Int2,
        lr_y -> Int2,
        inserted_at -> Timestamptz,
    }
}

table! {
    subscription_masks (id) {
        id -> Int4,
        analysis_subscription_id -> Int4,
        ul_x -> Int2,
        ul_y -> Int2,
        lr_x -> Int2,
        lr_y -> Int2,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        timezone -> Varchar,
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

joinable!(analysis_instances -> analysis_engines (analysis_engine_id));
joinable!(analysis_subscriptions -> cameras (camera_id));
joinable!(cameras -> camera_groups (camera_group_id));
joinable!(observations -> video_units (video_unit_id));
joinable!(subscription_masks -> analysis_subscriptions (analysis_subscription_id));
joinable!(video_files -> video_units (video_unit_id));
joinable!(video_units -> cameras (camera_id));

allow_tables_to_appear_in_same_query!(
    analysis_engines,
    analysis_instances,
    analysis_subscriptions,
    camera_groups,
    cameras,
    observations,
    subscription_masks,
    users,
    video_files,
    video_units,
);
