extern crate askama;

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

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
