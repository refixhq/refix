//! The checked-in Python package generated from the toy stays fresh. The
//! Python package's tests read it end to end.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use refix_codegen::python::generate;
use refix_dictionary::quickfix;

const TOY_XML: &str = include_str!("data/toy.xml");

/// The checked-in package's modules by path.
fn checked_in() -> BTreeMap<PathBuf, String> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python/refix-engine/tests/toy_generated");
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

#[test]
fn the_checked_in_package_is_fresh() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let package = generate(&parsed.dictionary, "toy.xml").unwrap();

    let generated: BTreeMap<PathBuf, String> = package
        .modules
        .into_iter()
        .map(|module| (module.path, module.code))
        .collect();
    assert_eq!(generated, checked_in());
}

#[test]
fn the_toy_generates_without_warnings() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let package = generate(&parsed.dictionary, "toy.xml").unwrap();

    assert!(package.warnings.is_empty());
}
