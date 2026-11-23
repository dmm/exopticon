use std::{collections::HashSet, env, sync::LazyLock};

use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use regex::Regex;

use crate::{
    api::{
        auth::User,
        cameras::StreamSource,
        configuration::{
            CameraGroupInput, CameraInput, MetadataInput, ResourceInput, StorageGroupInput,
            UserInput,
        },
    },
    schema::{
        camera_group_memberships as memberships, camera_groups, cameras, storage_groups, users,
    },
};

use super::{Error, OtherError, Service, cameras::Camera, storage_groups::StorageGroup};

pub const ACTIVE: i32 = 0;
pub const AWAITING_CAPTURE_STOP: i32 = 1;
pub const READY_FOR_PURGE: i32 = 2;

const fn field(field: &'static str, message: &'static str) -> Error {
    Error::Field { field, message }
}

fn required(input: Option<String>, key: &'static str) -> Result<String, Error> {
    input
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| field(key, "Enter a value."))
}

fn validate_name(name: &str, key: &'static str) -> Result<(), Error> {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").expect("valid name regex"));
    if !NAME.is_match(name) {
        return Err(field(key, "Only snakecase is allowed."));
    }
    Ok(())
}

fn resource_name(
    existing: Option<String>,
    supplied: Option<String>,
    key: &'static str,
) -> Result<(String, bool), Error> {
    if let Some(name) = existing {
        if supplied.is_some() {
            return Err(field(key, "Name cannot be changed."));
        }
        return Ok((name, false));
    }
    let name = required(supplied, key)?;
    validate_name(&name, key)?;
    Ok((name, true))
}

fn display_name(value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|name| required(Some(name), "displayName"))
        .transpose()
}

fn set<T>(value: &mut T, input: Option<T>) {
    if let Some(input) = input {
        *value = input;
    }
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn validate_device_url(url: &str, key: &'static str) -> Result<(), Error> {
    if !url::Url::parse(url).is_ok_and(|url| crate::onvif_client::is_device_url(&url)) {
        return Err(field(
            key,
            "Enter an HTTP or HTTPS device service URL without credentials.",
        ));
    }
    Ok(())
}

fn hash_password(password: &str) -> Result<String, Error> {
    if password.len() > 72 {
        return Err(field(
            "password",
            "Password can be no longer than 72 bytes.",
        ));
    }
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|err| Error::Other(OtherError::PasswordHash(err)))
}

fn add_member(
    conn: &mut SqliteConnection,
    group: &str,
    camera: &str,
    position: i32,
) -> Result<(), Error> {
    diesel::insert_into(memberships::table)
        .values((
            memberships::camera_group_name.eq(group),
            memberships::camera_name.eq(camera),
            memberships::display_order.eq(position),
        ))
        .execute(conn)?;
    Ok(())
}

impl Service {
    pub fn bootstrap_user(&self) -> Result<(), Error> {
        self.bootstrap_user_with(|| {
            (
                env::var("EXOPTICON_BOOTSTRAP_USERNAME").ok(),
                env::var("EXOPTICON_BOOTSTRAP_PASSWORD").ok(),
            )
        })
    }

    pub(super) fn bootstrap_user_with(
        &self,
        credentials: impl FnOnce() -> (Option<String>, Option<String>),
    ) -> Result<(), Error> {
        db_write!(self, "bootstrap_user", |conn| {
            if users::table.count().get_result::<i64>(conn)? != 0 {
                return Ok(());
            }
            let (username, password) = credentials();
            let username = nonempty(username)
                .filter(|value| !value.trim().is_empty())
                .ok_or(field(
                    "EXOPTICON_BOOTSTRAP_USERNAME",
                    "Set this variable to bootstrap first user.",
                ))?;
            let password = nonempty(password).ok_or(field(
                "EXOPTICON_BOOTSTRAP_PASSWORD",
                "Set this variable to bootstrap first user.",
            ))?;
            validate_name(&username, "EXOPTICON_BOOTSTRAP_USERNAME")?;
            let hash = hash_password(&password)?;
            diesel::insert_into(users::table)
                .values((
                    users::username.eq(&username),
                    users::display_name.eq(&username),
                    users::password.eq(hash),
                ))
                .execute(conn)?;
            Ok(())
        })
    }

    pub fn list_users(&self) -> Result<Vec<User>, Error> {
        db_read!(self, "list_users", |conn| {
            let rows = users::table
                .order(users::username)
                .load::<super::auth::User>(conn)?;
            Ok(rows.into_iter().map(Into::into).collect())
        })
    }

    pub fn fetch_user(&self, username: &str) -> Result<User, Error> {
        db_read!(self, "fetch_user", |conn| {
            Ok(users::table
                .find(username)
                .first::<super::auth::User>(conn)?
                .into())
        })
    }

    pub fn save_user(&self, existing: Option<String>, input: UserInput) -> Result<(), Error> {
        let (name, create) = resource_name(existing, input.username, "username")?;
        let display = display_name(input.display_name)?;
        let password = nonempty(input.password);
        if create && password.is_none() {
            return Err(field("password", "Password must be provided."));
        }
        let hash = password.as_deref().map(hash_password).transpose()?;
        db_write!(self, "save_user", |conn| {
            let target = users::table.find(&name);
            if create {
                let inserted = diesel::insert_into(users::table)
                    .values((
                        users::username.eq(&name),
                        users::display_name.eq(display.as_ref().unwrap_or(&name)),
                        users::password.eq(hash.as_ref().expect("password required for creation")),
                    ))
                    .on_conflict(users::username)
                    .do_nothing()
                    .execute(conn)?;
                if inserted == 0 {
                    return Err(field("username", "This username is already in use."));
                }
            } else {
                let old = target.first::<super::auth::User>(conn)?;
                diesel::update(target)
                    .set((
                        users::display_name.eq(display.unwrap_or(old.display_name)),
                        users::password.eq(hash.unwrap_or(old.password)),
                    ))
                    .execute(conn)?;
            }
            Ok(())
        })
    }

    pub fn delete_user(&self, username: &str) -> Result<(), Error> {
        db_write!(self, "delete_user", |conn| {
            if diesel::delete(users::table.find(username)).execute(conn)? == 0 {
                return Err(Error::NotFound);
            }
            if users::table.count().get_result::<i64>(conn)? == 0 {
                return Err(Error::Conflict("Cannot delete last user."));
            }
            Ok(())
        })
    }

    pub fn save_storage_group(
        &self,
        existing: Option<String>,
        input: ResourceInput<StorageGroupInput>,
    ) -> Result<(), Error> {
        let (name, create) = resource_name(existing, input.metadata.name, "name")?;
        let display = display_name(input.metadata.display_name)?;
        let StorageGroupInput {
            storage_path,
            max_storage_size,
        } = input.spec;
        if max_storage_size.is_some_and(|size| !(0..=i64::MAX / (1024 * 1024)).contains(&size)) {
            return Err(field("maxStorageSize", "Enter a nonnegative quota in MiB."));
        }
        db_write!(self, "save_storage_group", |conn| {
            let target = storage_groups::table.find(&name);
            if create {
                let path = required(storage_path, "storagePath")?;
                if path.contains('\0') {
                    return Err(field("storagePath", "Enter a valid filesystem path."));
                }
                let size =
                    max_storage_size.ok_or(field("maxStorageSize", "Enter a quota in MiB."))?;
                let inserted = diesel::insert_into(storage_groups::table)
                    .values((
                        storage_groups::name.eq(&name),
                        storage_groups::display_name.eq(display.as_ref().unwrap_or(&name)),
                        storage_groups::storage_path.eq(path),
                        storage_groups::max_storage_size.eq(size),
                    ))
                    .on_conflict(storage_groups::name)
                    .do_nothing()
                    .execute(conn)?;
                if inserted == 0 {
                    return Err(field("name", "This storage group name is already in use."));
                }
            } else {
                if storage_path.is_some() {
                    return Err(field(
                        "storagePath",
                        "The storage path is fixed after creation.",
                    ));
                }
                let old = target.first::<StorageGroup>(conn)?;
                diesel::update(target)
                    .set((
                        storage_groups::display_name.eq(display.unwrap_or(old.display_name)),
                        storage_groups::max_storage_size
                            .eq(max_storage_size.unwrap_or(old.max_storage_size)),
                    ))
                    .execute(conn)?;
            }
            Ok(())
        })
    }

    pub fn delete_storage_group(&self, name: &str) -> Result<(), Error> {
        db_write!(self, "delete_storage_group", |conn| {
            if cameras::table
                .filter(cameras::storage_group_name.eq(name))
                .count()
                .get_result::<i64>(conn)?
                != 0
            {
                return Err(Error::Conflict(
                    "This storage group cannot be deleted while cameras are assigned to it or their recordings are still being deleted.",
                ));
            }
            if diesel::delete(storage_groups::table.find(name)).execute(conn)? == 0 {
                return Err(Error::NotFound);
            }
            Ok(())
        })
    }

    pub fn save_camera(
        &self,
        existing: Option<String>,
        input: ResourceInput<CameraInput>,
    ) -> Result<(), Error> {
        let (name, create) = resource_name(existing, input.metadata.name, "name")?;
        let display = display_name(input.metadata.display_name)?;
        let spec = input.spec;
        if !create && spec.storage_group_name.is_some() {
            return Err(field(
                "storageGroupName",
                "The storage group is fixed after camera creation.",
            ));
        }
        db_write!(self, "save_camera", |conn| {
            let target = cameras::table.find(&name);
            let mut camera = if create {
                if let Some(state) = target
                    .select(cameras::deletion_state)
                    .first::<i32>(conn)
                    .optional()?
                {
                    if state != ACTIVE {
                        return Err(Error::Conflict(
                            "That camera name is reserved until recording cleanup finishes.",
                        ));
                    }
                    return Err(field("name", "This camera name is already in use."));
                }
                Camera {
                    name: name.clone(),
                    display_name: name.clone(),
                    storage_group_name: required(spec.storage_group_name, "storageGroupName")?,
                    rtsp_url: String::new(),
                    username: String::new(),
                    password: String::new(),
                    onvif_profile_token: None,
                    enabled: false,
                    ptz_x_step_size: 1,
                    ptz_y_step_size: 1,
                    deletion_state: ACTIVE,
                    onvif_device_url: None,
                    ptz_enabled: false,
                    ptz_device_url: None,
                    ptz_profile_token: None,
                }
            } else {
                target
                    .filter(cameras::deletion_state.eq(ACTIVE))
                    .first::<Camera>(conn)?
            };

            set(&mut camera.display_name, display);
            set(&mut camera.username, nonempty(spec.username));
            set(&mut camera.password, nonempty(spec.password));
            set(&mut camera.enabled, spec.enabled);

            let source_changed = spec.stream_source.is_some();
            if let Some(source) = spec.stream_source {
                match source {
                    StreamSource::StaticRtsp { url } => {
                        if !url.trim().is_empty() {
                            if !url::Url::parse(&url).is_ok_and(|url| {
                                matches!(url.scheme(), "rtsp" | "rtsps") && url.has_host()
                            }) {
                                return Err(field(
                                    "rtspUrl",
                                    "Enter an RTSP or RTSPS URL with a host.",
                                ));
                            }
                            camera.rtsp_url = url;
                        }
                        camera.onvif_device_url = None;
                        camera.onvif_profile_token = None;
                    }
                    StreamSource::Onvif {
                        device_url,
                        profile_token,
                    } => {
                        validate_device_url(&device_url, "onvifDeviceUrl")?;
                        if profile_token.trim().is_empty() {
                            return Err(field("onvifProfileToken", "Enter a profile token."));
                        }
                        camera.onvif_device_url = Some(device_url);
                        camera.onvif_profile_token = Some(profile_token);
                    }
                }
            }
            if camera.onvif_device_url.is_none() && camera.rtsp_url.trim().is_empty() {
                return Err(field("rtspUrl", "Enter an RTSP URL."));
            }

            let ptz_changed = spec.ptz.is_some();
            if let Some(ptz) = spec.ptz {
                camera.ptz_enabled = ptz.is_some();
                if let Some(ptz) = ptz {
                    if let Some(url) = &ptz.device_url {
                        validate_device_url(url, "ptzDeviceUrl")?;
                    }
                    if ptz
                        .profile_token
                        .as_ref()
                        .is_some_and(|token| token.trim().is_empty())
                    {
                        return Err(field("ptzProfileToken", "Enter a profile token."));
                    }
                    camera.ptz_device_url = ptz.device_url;
                    camera.ptz_profile_token = ptz.profile_token;
                    camera.ptz_x_step_size = ptz.x_step_size;
                    camera.ptz_y_step_size = ptz.y_step_size;
                } else {
                    camera.ptz_device_url = None;
                    camera.ptz_profile_token = None;
                }
            }
            if camera.ptz_enabled
                && (source_changed || ptz_changed)
                && camera.onvif_device_url.is_none()
            {
                if camera.ptz_device_url.is_none() {
                    return Err(field("ptzDeviceUrl", "Enter a PTZ device service URL."));
                }
                if camera.ptz_profile_token.is_none() {
                    return Err(field("ptzProfileToken", "Enter a PTZ profile token."));
                }
            }

            if create
                && storage_groups::table
                    .find(&camera.storage_group_name)
                    .select(storage_groups::name)
                    .first::<String>(conn)
                    .optional()?
                    .is_none()
            {
                return Err(field(
                    "storageGroupName",
                    "Choose an existing storage group.",
                ));
            }

            if create {
                diesel::insert_into(cameras::table)
                    .values(&camera)
                    .execute(conn)?;
                let last: Option<i32> = memberships::table
                    .filter(memberships::camera_group_name.eq("all"))
                    .select(diesel::dsl::max(memberships::display_order))
                    .first(conn)?;
                add_member(conn, "all", &name, last.unwrap_or(-1).saturating_add(1))?;
            } else {
                diesel::update(target).set(&camera).execute(conn)?;
            }
            Ok(())
        })
    }

    pub fn request_camera_deletion(&self, name: &str) -> Result<(), Error> {
        db_write!(self, "request_camera_deletion", |conn| {
            let target = cameras::table.find(name);
            if target.select(cameras::deletion_state).first::<i32>(conn)? == ACTIVE {
                diesel::update(target)
                    .set(cameras::deletion_state.eq(AWAITING_CAPTURE_STOP))
                    .execute(conn)?;
                diesel::delete(memberships::table.filter(memberships::camera_name.eq(name)))
                    .execute(conn)?;
            }
            Ok(())
        })
    }

    pub fn release_pending_camera_deletions(&self) -> Result<(), Error> {
        db_write!(self, "release_pending_camera_deletions", |conn| {
            diesel::update(
                cameras::table.filter(cameras::deletion_state.eq(AWAITING_CAPTURE_STOP)),
            )
            .set(cameras::deletion_state.eq(READY_FOR_PURGE))
            .execute(conn)?;
            Ok(())
        })
    }

    pub fn save_camera_group(
        &self,
        existing: Option<String>,
        input: ResourceInput<CameraGroupInput>,
    ) -> Result<(), Error> {
        let ResourceInput {
            metadata:
                MetadataInput {
                    name,
                    display_name: display,
                },
            spec,
        } = input;
        let (name, create) = resource_name(existing, name, "name")?;
        let display = display_name(display)?;
        db_write!(self, "save_camera_group", |conn| {
            let target = camera_groups::table.find(&name);
            if create {
                let inserted = diesel::insert_into(camera_groups::table)
                    .values((
                        camera_groups::name.eq(&name),
                        camera_groups::display_name.eq(display.as_ref().unwrap_or(&name)),
                    ))
                    .on_conflict(camera_groups::name)
                    .do_nothing()
                    .execute(conn)?;
                if inserted == 0 {
                    return Err(field("name", "This camera group name is already in use."));
                }
            } else {
                let old = target
                    .select(camera_groups::display_name)
                    .first::<String>(conn)?;
                diesel::update(target)
                    .set(camera_groups::display_name.eq(display.unwrap_or(old)))
                    .execute(conn)?;
            }
            if let Some(members) = spec.members {
                let active: HashSet<String> = cameras::table
                    .filter(cameras::deletion_state.eq(ACTIVE))
                    .select(cameras::name)
                    .load::<String>(conn)?
                    .into_iter()
                    .collect();
                let unique: HashSet<_> = members.iter().collect();
                if unique.len() != members.len() {
                    return Err(field("members", "Each camera may appear only once."));
                }
                if members.iter().any(|camera| !active.contains(camera)) {
                    return Err(field(
                        "members",
                        "Choose existing cameras. Reload the group if a camera was deleted.",
                    ));
                }
                if name == "all" && active.len() != members.len() {
                    return Err(field(
                        "members",
                        "The all group must include every camera. Reload the group if a camera was added.",
                    ));
                }
                diesel::delete(memberships::table.filter(memberships::camera_group_name.eq(&name)))
                    .execute(conn)?;
                for (position, camera) in members.iter().enumerate() {
                    let position = i32::try_from(position)
                        .map_err(|_| field("members", "Too many cameras."))?;
                    add_member(conn, &name, camera, position)?;
                }
            }
            Ok(())
        })
    }

    pub fn delete_camera_group(&self, name: &str) -> Result<(), Error> {
        db_write!(self, "delete_camera_group", |conn| {
            if name == "all" {
                return Err(Error::Conflict("The all camera group cannot be deleted."));
            }
            if diesel::delete(camera_groups::table.find(name)).execute(conn)? == 0 {
                return Err(Error::NotFound);
            }
            Ok(())
        })
    }
}
