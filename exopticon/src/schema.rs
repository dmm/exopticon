/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

table! {
    alert_rule_cameras (id) {
        id -> Int4,
        alert_rule_id -> Int4,
        camera_id -> Int4,
    }
}

table! {
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

table! {
    alerts (id) {
        id -> Int4,
        alert_rule_id -> Int4,
        time -> Timestamptz,
        inserted_at -> Timestamptz,
    }
}

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
    notification_contacts (id) {
        id -> Int4,
        group_name -> Text,
        username -> Text,
    }
}

table! {
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

table! {
    observations (id) {
        id -> Int8,
        video_unit_id -> Int4,
        frame_offset -> Int8,
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

joinable!(alert_rule_cameras -> alert_rules (alert_rule_id));
joinable!(alert_rule_cameras -> cameras (camera_id));
joinable!(alert_rules -> analysis_instances (analysis_instance_id));
joinable!(alert_rules -> notifiers (notifier_id));
joinable!(alerts -> alert_rules (alert_rule_id));
joinable!(analysis_instances -> analysis_engines (analysis_engine_id));
joinable!(analysis_subscriptions -> cameras (camera_id));
joinable!(cameras -> camera_groups (camera_group_id));
joinable!(observations -> video_units (video_unit_id));
joinable!(subscription_masks -> analysis_subscriptions (analysis_subscription_id));
joinable!(video_files -> video_units (video_unit_id));
joinable!(video_units -> cameras (camera_id));

allow_tables_to_appear_in_same_query!(
    alert_rule_cameras,
    alert_rules,
    alerts,
    analysis_engines,
    analysis_instances,
    analysis_subscriptions,
    camera_groups,
    cameras,
    notification_contacts,
    notifiers,
    observations,
    subscription_masks,
    users,
    video_files,
    video_units,
);
