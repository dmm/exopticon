use actix::{Handler, Message};
use diesel::{self, prelude::*};
use errors::ServiceError;
use models::{
    Camera, CameraGroup, CameraGroupAndCameras, CreateCameraGroup, DbExecutor, FetchAllCameraGroup,
    FetchAllCameraGroupAndCameras, FetchCameraGroup, UpdateCameraGroup,
};
use schema::camera_groups::dsl::*;

impl Message for CreateCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<CreateCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: CreateCameraGroup, _: &mut Self::Context) -> Self::Result {
        use schema::camera_groups::dsl::*;
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
        use schema::camera_groups::dsl::*;
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
        use schema::camera_groups::dsl::*;
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
        use schema::cameras::dsl::*;
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
