//! The frontend handles the full, real QuickFIX dictionary files: parsing
//! must succeed, and anything not yet modelled surfaces as warnings.

use refix_dictionary::quickfix;
use refix_dictionary::{Protocol, Version};

const FIX44: &str = include_str!("data/quickfix/FIX44.xml");

#[test]
fn parses_the_full_fix44_dictionary() {
    let parsed = quickfix::parse(FIX44).unwrap();

    assert_eq!(
        parsed.dictionary.version,
        Version {
            protocol: Protocol::Fix,
            major: 4,
            minor: 4,
            service_pack: 0,
        }
    );
    assert!(parsed.warnings.is_empty());
}
