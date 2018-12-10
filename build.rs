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

    Command::new("make")
        .current_dir("src/cworkers")
        .status()
        .expect("c worker build failed.");

    if !cfg!(debug_assertions) {
        Command::new("npm")
            .current_dir("web")
            .arg("install")
            .status()
            .expect("fetching web assets failed.");

        Command::new("npm")
            .current_dir("web")
            .arg("run")
            .arg("deploy")
            .status()
            .expect("building web assets failed.");
    }
}
