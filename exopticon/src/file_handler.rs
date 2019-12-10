use crate::models::{FileExecutor, RemoveFile};

use actix::{Handler, Message};

impl Message for RemoveFile {
    type Result = Result<(), std::io::Error>;
}

impl Handler<RemoveFile> for FileExecutor {
    type Result = Result<(), std::io::Error>;

    fn handle(&mut self, msg: RemoveFile, _: &mut Self::Context) -> Self::Result {
        std::fs::remove_file(msg.path)
    }
}
