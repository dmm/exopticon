use std::time::Duration;

use bincode::{deserialize, serialize};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use exserial::models::{CameraFrame, FrameResolution, FrameSource};

extern crate exserial;

pub fn criterion_benchmark(c: &mut Criterion) {
    // make a vec of about 318KiB to represent a typical frame
    let mut jpeg: Vec<u8> = Vec::with_capacity(321900);
    // initialize with random values
    for _ in 0..jpeg.capacity() {
        jpeg.push(rand::random());
    }

    let frame = CameraFrame {
        camera_id: 8,
        jpeg,
        resolution: FrameResolution::HD,
        source: FrameSource::Camera { camera_id: 8 },
        offset: 0,
        unscaled_width: 1920,
        unscaled_height: 1080,
    };

    let serialized = serialize(&frame).expect("Unable to serialize message!");

    let mut group = c.benchmark_group("serialize deserialize throughput");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("bincode serialize frame", |b| {
        b.iter(|| {
            serialize(black_box(&frame)).expect("Unable to serialize message!");
        });
    });

    group.bench_function("bincode deserialize frame", |b| {
        b.iter(|| {
            let _frame: CameraFrame =
                deserialize(black_box(&serialized)).expect("Deserialization failed!");
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
