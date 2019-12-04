extern crate askama;

use std::process::Command;

fn main() {
    askama::rerun_if_templates_changed();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cworkers/captureworker.c");
    println!("cargo:rerun-if-changed=src/cworkers/exvid.c");
    println!("cargo:rerun-if-changed=src/cworkers/Makefile");
    println!("cargo:rerun-if-changed=src/cworkers/mpack.c");
    println!("cargo:rerun-if-changed=src/cworkers/mpack_frame.c");
    println!("cargo:rerun-if-changed=src/cworkers/playbackworker.c");
    println!("cargo:rerun-if-changed=src/cworkers/exlog.h");
    println!("cargo:rerun-if-changed=src/cworkers/exvid.h");
    println!("cargo:rerun-if-changed=src/cworkers/mpack-config.h");
    println!("cargo:rerun-if-changed=src/cworkers/mpack_frame.h");
    println!("cargo:rerun-if-changed=src/cworkers/mpack.h");
    println!("cargo:rerun-if-changed=src/cworkers/timing.h");
    println!("cargo:rerun-if-changed=web");

    assert!(Command::new("make")
        .current_dir("src/cworkers")
        .status()
        .expect("c worker build execute failed.")
        .success());

    if !cfg!(debug_assertions) {
        assert!(Command::new("npm")
            .current_dir("web")
            .arg("install")
            .status()
            .expect("fetching web assets failed.")
            .success());

        assert!(Command::new("npm")
            .current_dir("web")
            .arg("run")
            .arg("ng")
            .arg("build")
            .arg("--")
            .arg("--prod")
            .status()
            .expect("building web assets failed.")
            .success());
    }
}
