//! The frontend handles the full, real QuickFIX dictionary files: parsing
//! must succeed, and anything not yet modelled surfaces as warnings.

use refix_dictionary::quickfix::{self, Warning};
use refix_dictionary::{
    Category, DataType, EnumValue, Field, FieldRef, Message, Protocol, Version,
};

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
            values: vec![],
        }
    );

    let side = fields.iter().find(|field| field.name == "Side").unwrap();
    assert_eq!(side.tag, 54);
    assert_eq!(side.values.len(), 16);
    assert_eq!(
        side.values[0],
        EnumValue {
            value: "1".to_owned(),
            description: "BUY".to_owned(),
        }
    );

    let messages = &parsed.dictionary.messages;
    assert_eq!(messages.len(), 93);
    assert_eq!(
        messages[0],
        Message {
            name: "Heartbeat".to_owned(),
            msg_type: "0".to_owned(),
            fields: vec![FieldRef {
                tag: 112,
                is_required: false,
            }],
            category: Category::Admin,
        }
    );

    // 3 unmodelled sections, 390 component references and 1 group across
    // the messages.
    assert_eq!(parsed.warnings.len(), 394);
    assert_eq!(
        parsed.warnings[0],
        Warning::UnsupportedSection {
            section: "header".to_owned(),
        }
    );
}
