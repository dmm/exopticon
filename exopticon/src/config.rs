/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2026 David Matthew Mattli <dmm@mattli.us>
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

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

pub const DEFAULT_CONFIG_PATH: &str = "/etc/exopticon/exopticon.toml";
pub const CONFIG_PATH_ENV: &str = "EXOPTICON_CONFIG";

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: String,
        source: toml::de::Error,
    },
    #[error("invalid config: {0}")]
    Validation(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RawConfig {
    #[serde(default)]
    storage_groups: Vec<StorageGroupConfig>,
    #[serde(default)]
    cameras: Vec<CameraConfig>,
    #[serde(default)]
    camera_groups: Vec<CameraGroupConfig>,
    #[serde(default)]
    users: Vec<UserConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedConfig {
    pub storage_groups: Vec<StorageGroup>,
    pub cameras: Vec<Camera>,
    pub camera_groups: Vec<CameraGroup>,
    pub users: Vec<User>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageGroup {
    pub name: String,
    pub display_name: String,
    pub storage_path: String,
    pub max_storage_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    pub name: String,
    pub display_name: String,
    pub storage_group_name: String,
    pub ip: String,
    pub onvif_port: i32,
    pub mac: String,
    pub username: String,
    pub password: String,
    pub rtsp_url: String,
    pub ptz_type: String,
    pub ptz_profile_token: String,
    pub enabled: bool,
    pub ptz_x_step_size: i16,
    pub ptz_y_step_size: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraGroup {
    pub name: String,
    pub display_name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct StorageGroupConfig {
    name: String,
    display_name: Option<String>,
    #[serde(rename = "path", alias = "storage-path")]
    storage_path: String,
    #[serde(rename = "max-storage-space", alias = "max-storage-size")]
    max_storage_size: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CameraConfig {
    name: String,
    display_name: Option<String>,
    #[serde(rename = "storage-group", alias = "storage-group-name")]
    storage_group_name: String,
    ip: String,
    onvif_port: i32,
    mac: String,
    username: String,
    password: String,
    rtsp_url: String,
    ptz_type: String,
    ptz_profile_token: String,
    #[serde(default)]
    enabled: bool,
    #[serde(rename = "ptz-step-size-x", alias = "ptz-x-step-size")]
    ptz_x_step_size: i16,
    #[serde(rename = "ptz-step-size-y", alias = "ptz-y-step-size")]
    ptz_y_step_size: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CameraGroupConfig {
    name: String,
    display_name: Option<String>,
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct UserConfig {
    username: String,
    display_name: Option<String>,
    password_hash: String,
}

impl ValidatedConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let path_text = path.display().to_string();
        let contents = std::fs::read_to_string(path).map_err(|source| Error::Read {
            path: path_text.clone(),
            source,
        })?;

        let config: RawConfig = toml::from_str(&contents).map_err(|source| Error::Parse {
            path: path_text,
            source,
        })?;

        config.validate()
    }

    #[cfg(test)]
    fn parse(contents: &str) -> Result<Self, Error> {
        let config: RawConfig = toml::from_str(contents).map_err(|source| Error::Parse {
            path: "<string>".to_string(),
            source,
        })?;

        config.validate()
    }
}

impl RawConfig {
    fn validate(self) -> Result<ValidatedConfig, Error> {
        validate_unique_names(
            "storage group",
            self.storage_groups.iter().map(|group| group.name.as_str()),
        )?;
        validate_unique_names(
            "camera",
            self.cameras.iter().map(|camera| camera.name.as_str()),
        )?;
        validate_unique_names(
            "camera group",
            self.camera_groups.iter().map(|group| group.name.as_str()),
        )?;
        validate_unique_names("user", self.users.iter().map(|user| user.username.as_str()))?;

        let storage_group_names: HashSet<&str> = self
            .storage_groups
            .iter()
            .map(|group| group.name.as_str())
            .collect();
        let camera_names: HashSet<&str> = self
            .cameras
            .iter()
            .map(|camera| camera.name.as_str())
            .collect();

        for camera in &self.cameras {
            if !storage_group_names.contains(camera.storage_group_name.as_str()) {
                return Err(Error::Validation(format!(
                    "camera '{}' references missing storage group '{}'",
                    camera.name, camera.storage_group_name
                )));
            }
        }

        for group in &self.camera_groups {
            validate_unique_members(&group.name, &group.members)?;
            for member in &group.members {
                if !camera_names.contains(member.as_str()) {
                    return Err(Error::Validation(format!(
                        "camera group '{}' references missing camera '{}'",
                        group.name, member
                    )));
                }
            }
        }

        for user in &self.users {
            validate_bcrypt_hash(&user.username, &user.password_hash)?;
        }

        let storage_groups = self
            .storage_groups
            .into_iter()
            .map(|group| StorageGroup {
                display_name: group.display_name.unwrap_or_else(|| group.name.clone()),
                name: group.name,
                storage_path: group.storage_path,
                max_storage_size: group.max_storage_size,
            })
            .collect();

        let cameras: Vec<Camera> = self
            .cameras
            .into_iter()
            .map(|camera| Camera {
                display_name: camera.display_name.unwrap_or_else(|| camera.name.clone()),
                name: camera.name,
                storage_group_name: camera.storage_group_name,
                ip: camera.ip,
                onvif_port: camera.onvif_port,
                mac: camera.mac,
                username: camera.username,
                password: camera.password,
                rtsp_url: camera.rtsp_url,
                ptz_type: camera.ptz_type,
                ptz_profile_token: camera.ptz_profile_token,
                enabled: camera.enabled,
                ptz_x_step_size: camera.ptz_x_step_size,
                ptz_y_step_size: camera.ptz_y_step_size,
            })
            .collect();

        let camera_names_in_order: Vec<String> =
            cameras.iter().map(|camera| camera.name.clone()).collect();
        let mut camera_groups: Vec<CameraGroup> = self
            .camera_groups
            .into_iter()
            .map(|group| CameraGroup {
                display_name: group.display_name.unwrap_or_else(|| group.name.clone()),
                name: group.name,
                members: group.members,
            })
            .collect();
        normalize_all_group(&mut camera_groups, &camera_names_in_order);

        let users = self
            .users
            .into_iter()
            .map(|user| User {
                display_name: user.display_name.unwrap_or_else(|| user.username.clone()),
                username: user.username,
                password_hash: user.password_hash,
            })
            .collect();

        Ok(ValidatedConfig {
            storage_groups,
            cameras,
            camera_groups,
            users,
        })
    }
}

fn validate_unique_names<'a>(
    resource_type: &str,
    names: impl Iterator<Item = &'a str>,
) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for name in names {
        validate_kebab_case(resource_type, name)?;
        if !seen.insert(name) {
            return Err(Error::Validation(format!(
                "duplicate {resource_type} name '{name}'"
            )));
        }
    }
    Ok(())
}

fn validate_kebab_case(resource_type: &str, name: &str) -> Result<(), Error> {
    static KEBAB_CASE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").expect("valid regex"));

    if KEBAB_CASE.is_match(name) {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "{resource_type} name '{name}' must be kebab-case"
        )))
    }
}

fn validate_unique_members(group_name: &str, members: &[String]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for member in members {
        if !seen.insert(member.as_str()) {
            return Err(Error::Validation(format!(
                "camera group '{group_name}' has duplicate member '{member}'"
            )));
        }
    }
    Ok(())
}

fn validate_bcrypt_hash(username: &str, password_hash: &str) -> Result<(), Error> {
    static BCRYPT_HASH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\$2[aby]\$(0[4-9]|[12][0-9]|3[01])\$[./A-Za-z0-9]{53}$").expect("valid regex")
    });

    if BCRYPT_HASH.is_match(password_hash) {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "user '{username}' must provide a bcrypt password-hash"
        )))
    }
}

fn normalize_all_group(camera_groups: &mut Vec<CameraGroup>, camera_names_in_order: &[String]) {
    if let Some(all_group) = camera_groups.iter_mut().find(|group| group.name == "all") {
        let mut members = all_group.members.clone();
        let mut seen: HashSet<String> = members.iter().cloned().collect();
        for camera_name in camera_names_in_order {
            if seen.insert(camera_name.clone()) {
                members.push(camera_name.clone());
            }
        }
        all_group.members = members;
    } else {
        camera_groups.push(CameraGroup {
            name: "all".to_string(),
            display_name: "all".to_string(),
            members: camera_names_in_order.to_vec(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{CameraGroup, Error, ValidatedConfig};

    const VALID_HASH: &str = "$2b$12$abcdefghijklmnopqrstuu5sDoo7w6PDtgfC5WbJg6Yt4RxKsKQyK";

    fn valid_config() -> String {
        format!(
            r#"
[[storage-groups]]
name = "default"
path = "/mnt/video"
max-storage-space = 1000

[[cameras]]
name = "garage-north"
display-name = "Garage North"
storage-group = "default"
ip = "192.168.1.108"
onvif-port = 80
mac = "9C-BE-CD-0A-52-5D"
username = "admin"
password = ""
rtsp-url = "rtsp://admin:@192.168.1.108:5544/live0.264"
ptz-type = "none"
ptz-profile-token = ""
enabled = true
ptz-step-size-x = 10
ptz-step-size-y = 10

[[cameras]]
name = "front-yard"
storage-group = "default"
ip = "192.168.1.109"
onvif-port = 80
mac = "9C-BE-CD-0A-52-5E"
username = "admin"
password = ""
rtsp-url = "rtsp://admin:@192.168.1.109:5544/live0.264"
ptz-type = "none"
ptz-profile-token = ""
enabled = true
ptz-step-size-x = 10
ptz-step-size-y = 10

[[camera-groups]]
name = "outside"
members = ["garage-north", "front-yard"]

[[users]]
username = "admin-user"
password-hash = "{VALID_HASH}"
"#
        )
    }

    #[test]
    fn parses_and_validates_config() {
        let config = ValidatedConfig::parse(&valid_config()).expect("parse and validate config");

        assert_eq!(config.storage_groups[0].display_name, "default");
        assert_eq!(config.cameras[0].display_name, "Garage North");
        assert_eq!(config.users[0].display_name, "admin-user");
        assert_eq!(
            config
                .camera_groups
                .iter()
                .find(|group| group.name == "all")
                .expect("all group")
                .members,
            vec!["garage-north".to_string(), "front-yard".to_string()]
        );
    }

    #[test]
    fn rejects_non_kebab_case_names() {
        let err = ValidatedConfig::parse(&valid_config().replace("garage-north", "Garage North"))
            .expect_err("validation should fail");

        assert!(matches!(err, Error::Validation(message) if message.contains("kebab-case")));
    }

    #[test]
    fn rejects_duplicate_names_within_resource_type() {
        let config = valid_config().replace("name = \"front-yard\"", "name = \"garage-north\"");
        let err = ValidatedConfig::parse(&config).expect_err("validation should fail");

        assert!(matches!(err, Error::Validation(message) if message.contains("duplicate camera")));
    }

    #[test]
    fn rejects_missing_storage_group_reference() {
        let config =
            valid_config().replace("storage-group = \"default\"", "storage-group = \"missing\"");
        let err = ValidatedConfig::parse(&config).expect_err("validation should fail");

        assert!(
            matches!(err, Error::Validation(message) if message.contains("missing storage group"))
        );
    }

    #[test]
    fn rejects_missing_camera_group_member_reference() {
        let config = valid_config().replace(
            "members = [\"garage-north\", \"front-yard\"]",
            "members = [\"garage-north\", \"missing-camera\"]",
        );
        let err = ValidatedConfig::parse(&config).expect_err("validation should fail");

        assert!(matches!(err, Error::Validation(message) if message.contains("missing camera")));
    }

    #[test]
    fn rejects_plaintext_user_passwords() {
        let config = valid_config().replace(VALID_HASH, "not-a-bcrypt-hash");
        let err = ValidatedConfig::parse(&config).expect_err("validation should fail");

        assert!(matches!(err, Error::Validation(message) if message.contains("bcrypt")));
    }

    #[test]
    fn explicit_all_group_controls_order_and_display_name() {
        let config = format!(
            r#"{}

[[camera-groups]]
name = "all"
display-name = "Everything"
members = ["front-yard"]
"#,
            valid_config()
        );

        let validated = ValidatedConfig::parse(&config).expect("parse and validate config");

        assert_eq!(
            validated
                .camera_groups
                .iter()
                .find(|group| group.name == "all"),
            Some(&CameraGroup {
                name: "all".to_string(),
                display_name: "Everything".to_string(),
                members: vec!["front-yard".to_string(), "garage-north".to_string()],
            })
        );
    }

    #[test]
    fn rejects_duplicate_camera_group_members() {
        let config = valid_config().replace(
            "members = [\"garage-north\", \"front-yard\"]",
            "members = [\"garage-north\", \"garage-north\"]",
        );
        let err = ValidatedConfig::parse(&config).expect_err("validation should fail");

        assert!(matches!(err, Error::Validation(message) if message.contains("duplicate member")));
    }
}
