use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed=exopticon.py");
    assert!(Command::new("cp")
        .arg("-r")
        .arg("exopticon.py")
        .arg("dist")
        .status()
        .expect("failed to copy exopticon.py")
        .success());

    println!("cargo:rerun-if-changed=cworkers/captureworker.c");
    println!("cargo:rerun-if-changed=cworkers/exlog.h");
    println!("cargo:rerun-if-changed=cworkers/exvid.c");
    println!("cargo:rerun-if-changed=cworkers/Makefile");
    println!("cargo:rerun-if-changed=cworkers/mpack_frame.c");
    println!("cargo:rerun-if-changed=cworkers/mpack_frame.h");
    println!("cargo:rerun-if-changed=cworkers/playbackworker.c");
    println!("cargo:rerun-if-changed=cworkers/timing.h");

    assert!(Command::new("make")
        .current_dir("cworkers")
        .status()
        .expect("c worker build execute failed.")
        .success());

    println!("cargo:rerun-if-changed=yolov4/build.bash");
    println!("cargo:rerun-if-changed=yolov4/darknet.py");
    println!("cargo:rerun-if-changed=yolov4/darknet.py");

    assert!(Command::new("bash")
        .arg("build.bash")
        .current_dir("yolov4")
        .status()
        .expect("failed building darknet")
        .success());

    println!("cargo:rerun-if-changed=frigate/build.bash");
    println!("cargo:rerun-if-changed=frigate/motion.py");

    assert!(Command::new("bash")
        .arg("build.bash")
        .current_dir("frigate")
        .status()
        .expect("failed building frigate motion")
        .success());
}
