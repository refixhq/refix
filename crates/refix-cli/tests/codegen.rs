//! The `refix codegen` command, run as a subprocess.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn toy_xml() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../refix-codegen/tests/data/toy.xml")
}

/// A fresh directory for one run's files.
fn scratch_directory() -> PathBuf {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let run = RUNS.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!("refix-cli-{}-{run}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

/// Runs `refix codegen` on the toy with `outputs` as (flag, name) pairs,
/// returning the directory the outputs are in.
fn codegen(outputs: &[(&str, &str)]) -> PathBuf {
    let directory = scratch_directory();
    let mut command = Command::new(env!("CARGO_BIN_EXE_refix"));
    command.arg("codegen").arg(toy_xml());
    for (flag, name) in outputs {
        command.arg(flag).arg(directory.join(name));
    }

    let status = command.status().unwrap();

    assert!(status.success());
    directory
}

/// The modules of the Python package at `root`, by path.
fn python_modules(root: &Path) -> BTreeMap<PathBuf, String> {
    let mut modules = BTreeMap::new();
    for folder in ["", "components", "messages"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "py") {
                let name = path.file_name().unwrap();
                let code = std::fs::read_to_string(&path).unwrap();
                modules.insert(Path::new(folder).join(name), code);
            }
        }
    }
    modules
}

fn python_golden() -> BTreeMap<PathBuf, String> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python/refix-engine/tests/toy_generated");
    python_modules(&root)
}

const RUST_GOLDEN: &str = include_str!("../../refix-codegen/tests/data/toy_generated.rs");

#[test]
fn writes_the_rust_module() {
    let directory = codegen(&[("--rust", "toy.rs")]);

    let written = std::fs::read_to_string(directory.join("toy.rs")).unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(written, RUST_GOLDEN);
}

#[test]
fn writes_the_python_package() {
    let directory = codegen(&[("--python", "toy")]);

    let written = python_modules(&directory.join("toy"));
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(written, python_golden());
}

#[test]
fn writes_every_output_from_one_run() {
    let directory = codegen(&[("--rust", "toy.rs"), ("--python", "toy")]);

    let rust = std::fs::read_to_string(directory.join("toy.rs")).unwrap();
    let python = python_modules(&directory.join("toy"));
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(rust, RUST_GOLDEN);
    assert_eq!(python, python_golden());
}

#[test]
fn a_python_directory_python_cannot_import_is_refused() {
    let directory = scratch_directory();

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(toy_xml())
        .arg("--python")
        .arg(directory.join("toy.py"))
        .output()
        .unwrap();
    let written = directory.join("toy.py").exists();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(!output.status.success());
    assert!(!written);
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: --python takes the directory of a package, and 'toy.py' is not a name python \
         can import\n"
    );
}

/// A dictionary whose `SecurityStatus` field and message share a name.
const CLASHING_XML: &str = r#"<fix type="FIX" major="5" minor="0" servicepack="2">
  <header>
    <field name="BeginString" required="Y"/>
  </header>
  <messages>
    <message name="SecurityStatus" msgtype="f" msgcat="app">
      <field name="SecurityStatus" required="N"/>
    </message>
  </messages>
  <components/>
  <fields>
    <field number="8" name="BeginString" type="STRING"/>
    <field number="965" name="SecurityStatus" type="STRING">
      <value enum="1" description="ACTIVE"/>
    </field>
  </fields>
</fix>
"#;

#[test]
fn labels_each_warning_with_its_language() {
    let directory = scratch_directory();
    let dictionary = directory.join("clash.xml");
    std::fs::write(&dictionary, CLASHING_XML).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(&dictionary)
        .arg("--rust")
        .arg(directory.join("clash.rs"))
        .arg("--python")
        .arg(directory.join("clash"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(output.status.success());
    let renamed = "`SecurityStatus` is taken by message 'SecurityStatus', so the enum of field \
                   'SecurityStatus' is named `SecurityStatusEnum`";
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!("warning: rust: {renamed}\nwarning: python: {renamed}\n")
    );
}

fn quickfix_xml(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionaries/quickfix")
        .join(name)
}

const NO_HEADER: &str = "warning: the dictionary has no header, which from FIX 5.0 is in a \
                         transport dictionary such as FIXT11.xml, given with --transport\n";

#[test]
fn joins_a_dictionary_with_its_transport() {
    let directory = scratch_directory();
    let module = directory.join("fix50.rs");

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(quickfix_xml("FIX50.xml"))
        .arg("--transport")
        .arg(quickfix_xml("FIXT11.xml"))
        .arg("--rust")
        .arg(&module)
        .output()
        .unwrap();
    let code = std::fs::read_to_string(&module).unwrap();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(output.status.success());
    assert!(code.starts_with(
        "// @generated by refix codegen from FIX50.xml and FIXT11.xml. Do not edit.\n"
    ));
    assert!(code.contains("pub struct Header<'a>("));
    assert!(code.contains("pub struct Logon(RawMessage);"));
    assert!(code.contains("pub struct NewOrderSingle(RawMessage);"));
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains(NO_HEADER)
    );
}

#[test]
fn a_dictionary_without_a_header_gets_a_warning() {
    let directory = scratch_directory();

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(quickfix_xml("FIX50.xml"))
        .arg("--rust")
        .arg(directory.join("fix50.rs"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains(NO_HEADER)
    );
}

#[test]
fn an_error_names_both_dictionaries() {
    let directory = scratch_directory();

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(toy_xml())
        .arg("--transport")
        .arg(quickfix_xml("FIXT11.xml"))
        .arg("--rust")
        .arg(directory.join("toy.rs"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("error: cannot parse '"));
    assert!(stderr.ends_with("FIXT11.xml': the application dictionary has a header of its own\n"));
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
