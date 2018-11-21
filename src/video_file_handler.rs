use actix::{Handler, Message};
use diesel::{self, prelude::*};
use errors::ServiceError;
use models::{CreateVideoFile, DbExecutor, UpdateVideoFile, VideoFile};

impl Message for CreateVideoFile {
    type Result = Result<VideoFile, ServiceError>;
}

impl Handler<CreateVideoFile> for DbExecutor {
    type Result = Result<VideoFile, ServiceError>;

    fn handle(&mut self, msg: CreateVideoFile, _: &mut Self::Context) -> Self::Result {
        use schema::video_files::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(video_files)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateVideoFile {
    type Result = Result<VideoFile, ServiceError>;
}

impl Handler<UpdateVideoFile> for DbExecutor {
    type Result = Result<VideoFile, ServiceError>;
    fn handle(&mut self, msg: UpdateVideoFile, _: &mut Self::Context) -> Self::Result {
        use schema::video_files::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(video_files.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
