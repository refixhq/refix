//! The checked-in Python module generated from the toy stays fresh. The
//! Python package's tests read it end to end.

use refix_codegen::python::generate;
use refix_dictionary::quickfix;

const TOY_XML: &str = include_str!("data/toy.xml");
const TOY_GENERATED: &str = include_str!("../../../python/refix-engine/tests/toy_generated.py");

#[test]
fn the_checked_in_module_is_fresh() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let generated = generate(&parsed.dictionary, "toy.xml").unwrap();
    assert_eq!(generated.code, TOY_GENERATED);
}

#[test]
fn the_toy_groups_are_reported_as_not_generated() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let generated = generate(&parsed.dictionary, "toy.xml").unwrap();

    let warnings: Vec<String> = generated.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "group 'NoPartyIDs' in component 'Parties' is not generated yet",
            "group 'NoMsgTypes' in message 'Logon' is not generated yet",
        ]
    );
}
