use std::process::Command;

fn main() {
    assert!(Command::new("bash")
        .arg("build.bash")
        .current_dir("yolov4")
        .status()
        .expect("failed building darknet")
        .success())
}
