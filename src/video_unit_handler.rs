use actix::{Handler, Message};
use diesel::{self, prelude::*};
use errors::ServiceError;
use models::{
    CreateVideoFile, CreateVideoUnit, CreateVideoUnitFile, DbExecutor, FetchBetweenVideoUnit,
    FetchVideoUnit, OutputVideoUnit, UpdateVideoFile, UpdateVideoUnit, UpdateVideoUnitFile,
    VideoFile, VideoUnit,
};

impl Message for CreateVideoUnit {
    type Result = Result<VideoUnit, ServiceError>;
}

impl Handler<CreateVideoUnit> for DbExecutor {
    type Result = Result<VideoUnit, ServiceError>;

    fn handle(&mut self, msg: CreateVideoUnit, _: &mut Self::Context) -> Self::Result {
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(video_units)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for CreateVideoUnitFile {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
}

impl Handler<CreateVideoUnitFile> for DbExecutor {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
    fn handle(&mut self, msg: CreateVideoUnitFile, _: &mut Self::Context) -> Self::Result {
        use schema::video_files::dsl::*;
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        // TODO: Wrap this in a transaction
        let video_unit: VideoUnit = diesel::insert_into(video_units)
            .values(CreateVideoUnit {
                camera_id: msg.camera_id,
                monotonic_index: msg.monotonic_index,
                begin_time: msg.begin_time,
                end_time: msg.begin_time,
            }).get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let video_file = diesel::insert_into(video_files)
            .values(CreateVideoFile {
                video_unit_id: video_unit.id,
                filename: msg.filename,
                size: 0,
            }).get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((video_unit, video_file))
    }
}

impl Message for UpdateVideoUnit {
    type Result = Result<VideoUnit, ServiceError>;
}

impl Handler<UpdateVideoUnit> for DbExecutor {
    type Result = Result<VideoUnit, ServiceError>;

    fn handle(&mut self, msg: UpdateVideoUnit, _: &mut Self::Context) -> Self::Result {
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(video_units.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateVideoUnitFile {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
}

impl Handler<UpdateVideoUnitFile> for DbExecutor {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
    fn handle(&mut self, msg: UpdateVideoUnitFile, _: &mut Self::Context) -> Self::Result {
        use schema::video_files::dsl::*;
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let video_unit = diesel::update(video_units)
            .set(UpdateVideoUnit {
                id: msg.video_unit_id,
                camera_id: None,
                monotonic_index: None,
                begin_time: None,
                end_time: Some(msg.end_time),
            }).get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let video_file = diesel::update(video_files)
            .set(UpdateVideoFile {
                id: msg.video_file_id,
                video_unit_id: None,
                filename: None,
                size: Some(msg.size),
            }).get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((video_unit, video_file))
    }
}

impl Message for FetchVideoUnit {
    type Result = Result<OutputVideoUnit, ServiceError>;
}

impl Handler<FetchVideoUnit> for DbExecutor {
    type Result = Result<OutputVideoUnit, ServiceError>;

    fn handle(&mut self, msg: FetchVideoUnit, _: &mut Self::Context) -> Self::Result {
        //        use schema::{video_files, video_units};
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let vu = video_units
            .find(msg.id)
            .get_result::<VideoUnit>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let files = ::models::VideoFile::belonging_to(&vu)
            .load::<VideoFile>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(OutputVideoUnit {
            id: vu.id,
            camera_id: vu.camera_id,
            monotonic_index: vu.monotonic_index,
            begin_time: vu.begin_time,
            end_time: vu.end_time,
            files: files,
            inserted_at: vu.inserted_at,
            updated_at: vu.updated_at,
        })
    }
}

impl Message for FetchBetweenVideoUnit {
    type Result = Result<Vec<VideoUnit>, ServiceError>;
}

impl Handler<FetchBetweenVideoUnit> for DbExecutor {
    type Result = Result<Vec<VideoUnit>, ServiceError>;

    fn handle(&mut self, msg: FetchBetweenVideoUnit, _: &mut Self::Context) -> Self::Result {
        use schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        video_units
            .filter(begin_time.le(msg.end_time))
            .filter(end_time.ge(msg.begin_time))
            .load::<VideoUnit>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
