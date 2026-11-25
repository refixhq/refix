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

/// The generated files at `root` with `extension`, by path.
fn modules(root: &Path, extension: &str) -> BTreeMap<PathBuf, String> {
    let mut modules = BTreeMap::new();
    for folder in ["", "components", "messages"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|found| found == extension) {
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
    modules(&root, "py")
}

fn rust_golden() -> BTreeMap<PathBuf, String> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../refix-codegen/tests/data/toy_generated");
    modules(&root, "rs")
}

#[test]
fn writes_the_rust_module_tree() {
    let directory = codegen(&[("--rust", "toy")]);

    let written = modules(&directory.join("toy"), "rs");
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(written, rust_golden());
}

#[test]
fn writes_the_python_package() {
    let directory = codegen(&[("--python", "toy")]);

    let written = modules(&directory.join("toy"), "py");
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(written, python_golden());
}

#[test]
fn writes_every_output_from_one_run() {
    let directory = codegen(&[("--rust", "rust/toy"), ("--python", "python/toy")]);

    let rust = modules(&directory.join("rust/toy"), "rs");
    let python = modules(&directory.join("python/toy"), "py");
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(rust, rust_golden());
    assert_eq!(python, python_golden());
}

#[test]
fn a_rust_directory_no_module_can_be_named_like_is_refused() {
    let directory = scratch_directory();

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(toy_xml())
        .arg("--rust")
        .arg(directory.join("toy.rs"))
        .output()
        .unwrap();
    let written = directory.join("toy.rs").exists();
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(!output.status.success());
    assert!(!written);
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: --rust takes the directory of a module, and 'toy.rs' is not a name a rust \
         module can have\n"
    );
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
        .arg(directory.join("rust/clash"))
        .arg("--python")
        .arg(directory.join("python/clash"))
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
    let module = directory.join("fix50");

    let output = Command::new(env!("CARGO_BIN_EXE_refix"))
        .arg("codegen")
        .arg(quickfix_xml("FIX50.xml"))
        .arg("--transport")
        .arg(quickfix_xml("FIXT11.xml"))
        .arg("--rust")
        .arg(&module)
        .output()
        .unwrap();
    let code = |file: &str| std::fs::read_to_string(module.join(file)).unwrap();
    let (root, header) = (code("mod.rs"), code("components/header.rs"));
    let (logon, order) = (
        code("messages/logon.rs"),
        code("messages/new_order_single.rs"),
    );
    std::fs::remove_dir_all(&directory).unwrap();

    assert!(output.status.success());
    assert!(root.starts_with(
        "// @generated by refix codegen from FIX50.xml and FIXT11.xml. Do not edit.\n"
    ));
    assert!(header.contains("pub struct Header<'a>("));
    assert!(logon.contains("pub struct Logon(RawMessage);"));
    assert!(order.contains("pub struct NewOrderSingle(RawMessage);"));
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
        .arg(directory.join("fix50"))
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
        .arg(directory.join("toy"))
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
