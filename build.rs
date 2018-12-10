extern crate askama;

use std::process::Command;

fn main() {
    askama::rerun_if_templates_changed();

    println!("cargo:rerun-if-changed=\"src/cworkers\"");

    Command::new("make")
        .current_dir("src/cworkers")
        .status()
        .expect("c worker build failed.");

    println!("cargo:rerun-if-changed=\"web/js\"");
    println!("cargo:rerun-if-changed=\"web/css\"");

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
