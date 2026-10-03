//! The frontend handles the full, real QuickFIX dictionary files: parsing
//! must succeed, and anything not yet modelled surfaces as warnings.

use refix_dictionary::quickfix::{self, Warning};
use refix_dictionary::{
    Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Member, Message,
    Protocol, Tag, Version, dictionary,
};

const FIX44: &str = include_str!("data/quickfix/FIX44.xml");

#[test]
fn parses_the_full_fix44_dictionary() {
    let parsed = quickfix::parse(FIX44).unwrap();

    assert_eq!(
        parsed.dictionary.version(),
        Version {
            protocol: Protocol::Fix,
            major: 4,
            minor: 4,
            service_pack: 0,
        }
    );

    let fields = parsed.dictionary.fields();
    assert_eq!(fields.len(), 912);
    assert_eq!(
        fields[0],
        Field {
            name: "Account".to_owned(),
            tag: Tag(1),
            data_type: DataType::String,
            values: vec![],
        }
    );

    let side = fields.iter().find(|field| field.name == "Side").unwrap();
    assert_eq!(side.tag, Tag(54));
    assert_eq!(side.values.len(), 16);
    assert_eq!(
        side.values[0],
        EnumValue {
            value: "1".to_owned(),
            description: "BUY".to_owned(),
        }
    );

    let messages = &parsed.dictionary.spec().messages;
    assert_eq!(messages.len(), 93);
    assert_eq!(
        messages[0],
        Message {
            name: "Heartbeat".to_owned(),
            msg_type: "0".to_owned(),
            members: vec![Member::Field(FieldRef {
                tag: Tag(112),
                is_required: false,
            })],
            category: Category::Admin,
        }
    );

    let components = &parsed.dictionary.spec().components;
    assert_eq!(components.len(), 104);
    assert_eq!(
        components[0],
        Component {
            name: "CommissionData".to_owned(),
            members: [12, 13, 479, 497]
                .into_iter()
                .map(|tag| {
                    Member::Field(FieldRef {
                        tag: Tag(tag),
                        is_required: false,
                    })
                })
                .collect(),
        }
    );

    let new_order_single = messages
        .iter()
        .find(|message| message.name == "NewOrderSingle")
        .unwrap();
    assert!(
        new_order_single
            .members
            .contains(&Member::Component(ComponentRef {
                name: "Instrument".to_owned(),
                is_required: true,
            }))
    );

    let resolved = parsed
        .dictionary
        .messages()
        .find(|message| message.name() == "NewOrderSingle")
        .unwrap();
    // 149 fields and 7 groups, components expanded in place.
    assert_eq!(resolved.members().count(), 156);
    // Symbol(55) arrives through the Instrument component's expansion.
    assert!(resolved.members().any(|member| matches!(
        member,
        dictionary::Member::Field { field, .. } if field.tag == Tag(55) && field.name == "Symbol"
    )));

    // Parties' NoPartyIDs, with NoPartySubIDs nested in its instances.
    let parties = resolved
        .members()
        .find_map(|member| match member {
            dictionary::Member::Group(group) if group.count_field().tag == Tag(453) => Some(group),
            _ => None,
        })
        .unwrap();
    assert!(!parties.is_required());
    assert_eq!(parties.delimiter().tag, Tag(448));
    let instance: Vec<(Tag, bool)> = parties
        .members()
        .map(|member| match member {
            dictionary::Member::Field { field, is_required } => (field.tag, is_required),
            dictionary::Member::Group(group) => (group.count_field().tag, group.is_required()),
        })
        .collect();
    assert_eq!(
        instance,
        vec![
            (Tag(448), true),
            (Tag(447), false),
            (Tag(452), false),
            (Tag(802), false)
        ]
    );

    // Every message-level group parses and resolves.
    let groups: usize = parsed
        .dictionary
        .messages()
        .map(|message| {
            message
                .members()
                .filter(|member| matches!(member, dictionary::Member::Group(_)))
                .count()
        })
        .sum();
    assert_eq!(groups, 356);

    // Only the header and trailer remain unmodelled.
    assert_eq!(
        parsed.warnings,
        vec![
            Warning::UnsupportedSection {
                section: "header".to_owned(),
            },
            Warning::UnsupportedSection {
                section: "trailer".to_owned(),
            },
        ]
    );
}
