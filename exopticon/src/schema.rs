// @generated automatically by Diesel CLI.

diesel::table! {
    alert_rule_cameras (id) {
        id -> Int4,
        alert_rule_id -> Int4,
        camera_id -> Int4,
    }
}

diesel::table! {
    alert_rules (id) {
        id -> Int4,
        name -> Varchar,
        analysis_instance_id -> Int4,
        tag -> Varchar,
        details -> Varchar,
        min_score -> Int2,
        min_cluster_size -> Int2,
        cool_down_time -> Int8,
        notifier_id -> Int4,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
        contact_group -> Varchar,
    }
}

diesel::table! {
    alerts (id) {
        id -> Int4,
        alert_rule_id -> Int4,
        time -> Timestamptz,
        inserted_at -> Timestamptz,
    }
}

diesel::table! {
    analysis_engines (id) {
        id -> Int4,
        name -> Text,
        version -> Text,
        entry_point -> Text,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
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

diesel::table! {
    analysis_subscriptions (id) {
        id -> Int4,
        analysis_instance_id -> Int4,
        camera_id -> Nullable<Int4>,
        source_analysis_instance_id -> Nullable<Int4>,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    camera_group_memberships (id) {
        id -> Int4,
        camera_group_id -> Int4,
        camera_id -> Int4,
        display_order -> Int4,
    }
}

diesel::table! {
    camera_groups (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::table! {
    cameras (id) {
        id -> Int4,
        storage_group_id -> Int4,
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
        ptz_x_step_size -> Int2,
        ptz_y_step_size -> Int2,
    }
}

diesel::table! {
    event_observations (id) {
        id -> Int8,
        event_id -> Uuid,
        observation_id -> Int8,
    }
}

diesel::table! {
    events (id) {
        id -> Uuid,
        tag -> Text,
        camera_id -> Int4,
        begin_time -> Timestamptz,
        end_time -> Timestamptz,
        display_observation_id -> Int8,
    }
}

diesel::table! {
    notification_contacts (id) {
        id -> Int4,
        group_name -> Text,
        username -> Text,
    }
}

diesel::table! {
    notifiers (id) {
        id -> Int4,
        name -> Varchar,
        hostname -> Varchar,
        port -> Int4,
        username -> Nullable<Varchar>,
        password -> Nullable<Varchar>,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    observation_snapshots (observation_id) {
        observation_id -> Int8,
        snapshot_path -> Text,
        snapshot_size -> Int4,
    }
}

diesel::table! {
    observations (id) {
        id -> Int8,
        frame_offset -> Int8,
        tag -> Text,
        details -> Text,
        score -> Int2,
        ul_x -> Int2,
        ul_y -> Int2,
        lr_x -> Int2,
        lr_y -> Int2,
        inserted_at -> Timestamptz,
        video_unit_id -> Uuid,
    }
}

diesel::table! {
    storage_groups (id) {
        id -> Int4,
        name -> Varchar,
        storage_path -> Varchar,
        max_storage_size -> Int8,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
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

diesel::table! {
    user_sessions (id) {
        id -> Int4,
        name -> Text,
        user_id -> Int4,
        session_key -> Text,
        is_token -> Bool,
        expiration -> Timestamptz,
        inserted_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        timezone -> Varchar,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    video_files (id) {
        id -> Int4,
        filename -> Varchar,
        size -> Int4,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
        video_unit_id -> Uuid,
    }
}

diesel::table! {
    video_units (id) {
        camera_id -> Int4,
        monotonic_index -> Int4,
        begin_time -> Timestamp,
        end_time -> Timestamp,
        inserted_at -> Timestamp,
        updated_at -> Timestamp,
        id -> Uuid,
    }
}

diesel::joinable!(alert_rule_cameras -> alert_rules (alert_rule_id));
diesel::joinable!(alert_rule_cameras -> cameras (camera_id));
diesel::joinable!(alert_rules -> analysis_instances (analysis_instance_id));
diesel::joinable!(alert_rules -> notifiers (notifier_id));
diesel::joinable!(alerts -> alert_rules (alert_rule_id));
diesel::joinable!(analysis_instances -> analysis_engines (analysis_engine_id));
diesel::joinable!(analysis_subscriptions -> cameras (camera_id));
diesel::joinable!(camera_group_memberships -> camera_groups (camera_group_id));
diesel::joinable!(camera_group_memberships -> cameras (camera_id));
diesel::joinable!(cameras -> storage_groups (storage_group_id));
diesel::joinable!(event_observations -> events (event_id));
diesel::joinable!(event_observations -> observations (observation_id));
diesel::joinable!(events -> cameras (camera_id));
diesel::joinable!(events -> observations (display_observation_id));
diesel::joinable!(observation_snapshots -> observations (observation_id));
diesel::joinable!(observations -> video_units (video_unit_id));
diesel::joinable!(subscription_masks -> analysis_subscriptions (analysis_subscription_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(video_files -> video_units (video_unit_id));
diesel::joinable!(video_units -> cameras (camera_id));

diesel::allow_tables_to_appear_in_same_query!(
    alert_rule_cameras,
    alert_rules,
    alerts,
    analysis_engines,
    analysis_instances,
    analysis_subscriptions,
    camera_group_memberships,
    camera_groups,
    cameras,
    event_observations,
    events,
    notification_contacts,
    notifiers,
    observation_snapshots,
    observations,
    storage_groups,
    subscription_masks,
    user_sessions,
    users,
    video_files,
    video_units,
);
