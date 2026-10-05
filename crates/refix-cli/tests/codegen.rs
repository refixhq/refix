//! The `refix codegen` command, run as a subprocess.

use std::path::PathBuf;
use std::process::Command;

fn toy_xml() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../refix-codegen/tests/data/toy.xml")
}

#[test]
fn writes_the_rust_module() {
    let output = std::env::temp_dir().join(format!("refix-cli-{}.rs", std::process::id()));

    let status = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(toy_xml())
        .arg("--rust")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    let written = std::fs::read_to_string(&output).unwrap();
    std::fs::remove_file(&output).unwrap();
    assert_eq!(
        written,
        include_str!("../../refix-codegen/tests/data/toy_generated.rs")
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
