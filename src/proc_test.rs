extern crate futures;
extern crate rmp;
extern crate rmp_serde;
#[macro_use]
extern crate serde;
extern crate serde_bytes;
#[macro_use]
extern crate serde_derive;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_process;

use std::process::{Command, Stdio};
use std::time::Instant;

use futures::{Future, Stream};
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio_codec::Framed;
use tokio_io::codec::length_delimited;
use tokio_process::{Child, CommandExt};

#[derive(Default, Debug, PartialEq, Deserialize, Serialize)]
struct CaptureMessage {
    #[serde(rename = "type")]
    #[serde(default)]
    pub message_type: String,

    #[serde(default)]
    pub level: String,

    #[serde(default)]
    pub message: String,

    #[serde(rename = "jpegFrame")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub jpeg: Vec<u8>,

    #[serde(rename = "jpegFrameScaled")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub scaled_jpeg: Vec<u8>,

    #[serde(default)]
    pub offset: i64,

    #[serde(default)]
    pub height: i32,

    #[serde(default)]
    pub filename: String,

    #[serde(rename = "beginTime")]
    #[serde(default)]
    pub begin_time: String,

    #[serde(rename = "endTime")]
    #[serde(default)]
    pub end_time: String,
}

fn print_lines(mut cat: Child) -> Box<Future<Item = (), Error = ()> + Send + 'static> {
    let stdout = cat.stdout().take().unwrap();
    let framed_stdout = length_delimited::FramedRead::new(stdout);

    let cycle = framed_stdout.for_each(|line| {
        let now = Instant::now();
        let mut de = Deserializer::new(&line[..]);

        let log_frame: Result<CaptureMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match log_frame {
            Ok(l) => {}
            Err(e) => println!("Error! {}", e),
        }

        Ok(())
    });

    let future = cycle.join(cat).map(|_| ()).map_err(|e| panic!("{}", e));

    Box::new(future)
}

fn main() {
    let mut cmd = Command::new("cat");
    cmd.arg("capturedata.dat");
    cmd.stdout(Stdio::piped());

    let now = Instant::now();

    let future = print_lines(cmd.spawn_async().expect("failed to spawn command"));
    tokio::run(future);

    println!("elapsed: {}", now.elapsed().as_secs());
}
