//! The `refix codegen` command, run as a subprocess.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn toy_xml() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../refix-codegen/tests/data/toy.xml")
}

/// Runs `refix codegen` on the toy with `outputs` as (flag, file name)
/// pairs, returning each written module.
fn codegen(outputs: &[(&str, &str)]) -> Vec<String> {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let run = RUNS.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!("refix-cli-{}-{run}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_refix"));
    command.arg("codegen").arg(toy_xml());
    for (flag, name) in outputs {
        command.arg(flag).arg(directory.join(name));
    }

    let status = command.status().unwrap();

    assert!(status.success());
    let written = outputs
        .iter()
        .map(|(_, name)| std::fs::read_to_string(directory.join(name)).unwrap())
        .collect();
    std::fs::remove_dir_all(&directory).unwrap();
    written
}

const RUST_GOLDEN: &str = include_str!("../../refix-codegen/tests/data/toy_generated.rs");
const PYTHON_GOLDEN: &str = include_str!("../../../python/refix-engine/tests/toy_generated.py");

#[test]
fn writes_the_rust_module() {
    assert_eq!(codegen(&[("--rust", "toy.rs")]), [RUST_GOLDEN]);
}

#[test]
fn writes_the_python_module() {
    assert_eq!(codegen(&[("--python", "toy.py")]), [PYTHON_GOLDEN]);
}

#[test]
fn writes_every_output_from_one_run() {
    assert_eq!(
        codegen(&[("--rust", "toy.rs"), ("--python", "toy.py")]),
        [RUST_GOLDEN, PYTHON_GOLDEN]
    );
}

#[test]
fn an_output_is_required() {
    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(toy_xml())
        .output()
        .unwrap();

    assert!(!output.status.success());
}
