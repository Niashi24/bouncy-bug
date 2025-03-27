use std::process::Command;

fn main() {
    Command::new("cargo")
        .args(["playdate", "run", "--release", "-p", "game"])
        .spawn()
        .unwrap();
}
