//! The shipped demo plays through the real binary.

use std::path::PathBuf;
use std::process::{Command, Output};

const THE_END: &str = "~ La tua vita e la tua missione terminano qui ~";

fn example() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/high-pass")
}

fn oink(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oink"))
        .args(args)
        .output()
        .expect("the oink binary runs")
}

#[test]
fn the_high_pass_plays_to_an_ending() {
    let example = example();
    let output = oink(&[
        "run",
        example.to_str().unwrap(),
        "--seed",
        "7",
        "--choices",
        "1,1,1,1,1,1,1,1",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.is_empty(),
        "the demo must load without warnings:\n{stderr}"
    );
    assert!(
        stdout.starts_with("The High Pass\n=============\n"),
        "{stdout}"
    );
    assert!(stdout.contains("Snow on the high pass."), "{stdout}");
    assert!(stdout.contains("* logic sense: "), "{stdout}");
    assert!(stdout.contains("> 1\n"), "{stdout}");
    assert!(stdout.trim_end().ends_with(THE_END), "{stdout}");
}

#[test]
fn the_same_seed_gives_the_same_transcript() {
    let example = example();
    let args = [
        "run",
        example.to_str().unwrap(),
        "--seed",
        "11",
        "--choices",
        "1,1,1,1,1,1,1,1",
    ];
    let first = oink(&args);
    let second = oink(&args);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn a_pick_out_of_range_fails() {
    let example = example();
    let output = oink(&["run", example.to_str().unwrap(), "--choices", "9"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("choice 9 is out of range: this scene has 2 choices"),
        "{stderr}"
    );
}

#[test]
fn a_zero_pick_fails() {
    let example = example();
    let output = oink(&["run", example.to_str().unwrap(), "--choices", "0"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("choice numbers start at 1"), "{stderr}");
}

#[test]
fn a_missing_bundle_fails() {
    let output = oink(&["run", "no/such/bundle"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("is not a directory"), "{stderr}");
}
