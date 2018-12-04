use actix::{Handler, Message};
use diesel::{self, prelude::*};
use crate::errors::ServiceError;
use crate::models::{
    Camera, CameraGroup, CameraGroupAndCameras, CreateCameraGroup, DbExecutor, FetchAllCameraGroup,
    FetchAllCameraGroupAndCameras, FetchCameraGroup, FetchCameraGroupFiles, UpdateCameraGroup,
    VideoFile, VideoUnit,
};
use crate::schema::camera_groups::dsl::*;

impl Message for CreateCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<CreateCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: CreateCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(camera_groups)
            .values(&msg)
            .get_result(conn)
            .map_err(|error| {
                println!("ERROR HERE WOO");
                println!("{:#?}", error);
                ServiceError::InternalServerError
            })
    }
}

impl Message for UpdateCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<UpdateCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: UpdateCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        println!("{:?}", msg);
        diesel::update(camera_groups.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|error| {
                println!("ERROR HERE WOO");
                println!("{:#?}", error);
                ServiceError::InternalServerError
            })
    }
}

impl Message for FetchCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<FetchCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: FetchCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let group = camera_groups
            .filter(id.eq(msg.id))
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?
            .pop();

        match group {
            None => Err(ServiceError::NotFound),
            Some(g) => Ok(g),
        }
    }
}

impl Message for FetchAllCameraGroup {
    type Result = Result<Vec<CameraGroup>, ServiceError>;
}
impl Handler<FetchAllCameraGroup> for DbExecutor {
    type Result = Result<Vec<CameraGroup>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllCameraGroup, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        camera_groups
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchAllCameraGroupAndCameras {
    type Result = Result<Vec<CameraGroupAndCameras>, ServiceError>;
}

impl Handler<FetchAllCameraGroupAndCameras> for DbExecutor {
    type Result = Result<Vec<CameraGroupAndCameras>, ServiceError>;

    fn handle(
        &mut self,
        _msg: FetchAllCameraGroupAndCameras,
        _: &mut Self::Context,
    ) -> Self::Result {
        use diesel::prelude::*;
        use crate::schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let mut groups_and_cameras: Vec<CameraGroupAndCameras> = Vec::new();

        let groups = camera_groups
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        for g in groups {
            let c = cameras
                .filter(camera_group_id.eq(g.id))
                .load::<Camera>(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            groups_and_cameras.push(CameraGroupAndCameras { 0: g, 1: c });
        }

        Ok(groups_and_cameras)
    }
}

impl Message for FetchCameraGroupFiles {
    type Result = Result<(i64, i64, Vec<(Camera, (VideoUnit, VideoFile))>), ServiceError>;
}

impl Handler<FetchCameraGroupFiles> for DbExecutor {
    type Result = Result<(i64, i64, Vec<(Camera, (VideoUnit, VideoFile))>), ServiceError>;

    fn handle(&mut self, msg: FetchCameraGroupFiles, _: &mut Self::Context) -> Self::Result {
        use diesel::dsl::sum;
        use crate::schema::camera_groups;
        use crate::schema::cameras::dsl::*;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();

        let max_size = camera_groups
            .select(max_storage_size)
            .filter(camera_groups::columns::id.eq(msg.camera_group_id))
            .first(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let current_size = video_files
            .select(sum(size))
            .inner_join(video_units.inner_join(cameras))
            .filter(camera_group_id.eq(msg.camera_group_id))
            .first(conn)
            .map(|result| match result {
                Some(val) => val,
                None => 0,
            }).map_err(|_error| ServiceError::InternalServerError)?;

        let files = cameras
            .inner_join(video_units.inner_join(video_files))
            .filter(camera_group_id.eq(msg.camera_group_id))
            .filter(size.gt(0))
            .order(begin_time.asc())
            .limit(msg.count)
            .load(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((max_size, current_size, files))
    }
}
