use actix::{Handler, Message};
use diesel::*;
use errors::ServiceError;
use models::{Camera, CreateCamera, DbExecutor, FetchAllCamera, FetchCamera, UpdateCamera};

impl Message for CreateCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<CreateCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: CreateCamera, _: &mut Self::Context) -> Self::Result {
        use schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(cameras)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<UpdateCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: UpdateCamera, _: &mut Self::Context) -> Self::Result {
        use schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(cameras.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<FetchCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: FetchCamera, _: &mut Self::Context) -> Self::Result {
        use schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let c = cameras
            .find(msg.id)
            .get_result::<Camera>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(c)
    }
}

impl Message for FetchAllCamera {
    type Result = Result<Vec<Camera>, ServiceError>;
}

impl Handler<FetchAllCamera> for DbExecutor {
    type Result = Result<Vec<Camera>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllCamera, _: &mut Self::Context) -> Self::Result {
        use schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        cameras
            .load::<Camera>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
