//! The frontend handles the full, real QuickFIX dictionary files: parsing
//! must succeed, and anything not yet modelled surfaces as warnings.

use refix_dictionary::quickfix::{self, Warning};
use refix_dictionary::{DataType, Field, Protocol, Version};

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

    let fields = &parsed.dictionary.fields;
    assert_eq!(fields.len(), 912);
    assert_eq!(
        fields[0],
        Field {
            name: "Account".to_owned(),
            tag: 1,
            data_type: DataType::String,
        }
    );

    // 245 fields carry enum values, none of which are modelled yet.
    assert_eq!(parsed.warnings.len(), 245);
    assert_eq!(
        parsed.warnings[0],
        Warning::UnsupportedEnumValues {
            field: "AdvSide".to_owned(),
        }
    );
}
