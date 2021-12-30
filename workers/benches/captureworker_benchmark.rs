use std::env;
use std::process::{Command, Stdio};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::tempdir;

pub fn run_captureworker(input_filename: &str, hwaccel_type: &str) {
    let dir = tempdir().expect("Failed to create temp dir for output files.");
    let dir_path = dir
        .path()
        .to_str()
        .expect("Failed to convert dirname to string.");
    let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
    let testdata_path = format!("{}/../../../testdata", worker_path);
    Command::new(format!("{}/cworkers/captureworker", worker_path))
        .arg(format!("{}/{}", testdata_path, input_filename))
        .arg(dir_path)
        .arg(hwaccel_type)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to execute captureworker");
}

pub fn captureworker_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("captureworker");

    group.sample_size(10);

    group.bench_function("h264 baseline none", |b| {
        b.iter(|| run_captureworker("h264_baseline.mkv", "none"))
    });

    group.bench_function("h264 baseline vaapi", |b| {
        b.iter(|| run_captureworker("h264_baseline.mkv", "vaapi"))
    });

    group.bench_function("h264 high none", |b| {
        b.iter(|| run_captureworker("h264_high.mkv", "none"))
    });

    group.bench_function("h264 high vaapi", |b| {
        b.iter(|| run_captureworker("h264_high.mkv", "vaapi"))
    });

    group.finish();
}

criterion_group!(benches, captureworker_benchmark);
criterion_main!(benches);
