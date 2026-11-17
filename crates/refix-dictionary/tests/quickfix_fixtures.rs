//! The frontend handles the full, real QuickFIX dictionary files: parsing
//! must succeed, and anything not yet modelled surfaces as warnings.

use refix_dictionary::quickfix;
use refix_dictionary::spec::{
    Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Member, Message,
    Protocol, Version,
};
use refix_dictionary::{MemberContext, Tag, dictionary};

const FIX44: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../dictionaries/quickfix/FIX44.xml"
));

const FIX42: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../dictionaries/quickfix/FIX42.xml"
));

#[test]
fn parses_the_full_fix42_dictionary() {
    let parsed = quickfix::parse(FIX42).unwrap();

    assert_eq!(
        parsed.dictionary.version(),
        Version {
            protocol: Protocol::Fix,
            major: 4,
            minor: 2,
            service_pack: 0,
        }
    );
    assert_eq!(parsed.dictionary.fields().len(), 405);
    assert_eq!(parsed.dictionary.spec().messages.len(), 46);
    // FIX 4.2 has no components, so every group is declared in place.
    assert!(parsed.dictionary.spec().components.is_empty());

    // MassQuote's NoQuoteSets declares NoQuoteEntries in its instances.
    let quote_sets = parsed
        .dictionary
        .messages()
        .find(|message| message.name() == "MassQuote")
        .unwrap()
        .members()
        .find_map(|member| match member {
            dictionary::Member::Group(group) => Some(group),
            dictionary::Member::Field { .. } => None,
        })
        .unwrap();
    assert_eq!(quote_sets.count_field().name, "NoQuoteSets");
    let quote_entries = quote_sets
        .members()
        .find_map(|member| match member {
            dictionary::Member::Group(group) => Some(group),
            dictionary::Member::Field { .. } => None,
        })
        .unwrap();
    assert_eq!(quote_entries.count_field().name, "NoQuoteEntries");
    assert_eq!(
        quote_entries.declared_in(),
        &MemberContext::Message("MassQuote".to_owned()).group("NoQuoteSets")
    );

    // Nothing is left unmodelled.
    assert!(parsed.warnings.is_empty());
}

const FIX50: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../dictionaries/quickfix/FIX50.xml"
));

const FIXT11: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../dictionaries/quickfix/FIXT11.xml"
));

#[test]
fn joins_the_fix50_dictionary_with_its_transport() {
    let alone = quickfix::parse(FIX50).unwrap();
    assert_eq!(alone.dictionary.header().count(), 0);
    assert_eq!(alone.dictionary.spec().messages.len(), 93);

    let parsed = quickfix::parse_with_transport(FIX50, FIXT11).unwrap();
    let dictionary = &parsed.dictionary;

    assert_eq!(
        dictionary.version(),
        Version {
            protocol: Protocol::Fix,
            major: 5,
            minor: 0,
            service_pack: 0,
        }
    );

    // The envelope comes from the transport.
    let header: Vec<dictionary::Member> = dictionary.header().collect();
    assert_eq!(header.len(), 30);
    let Some(dictionary::Member::Group(hops)) = header.last() else {
        panic!("expected the header to end with a group");
    };
    assert_eq!(hops.count_field().name, "NoHops");
    assert_eq!(dictionary.trailer().count(), 3);

    // So do the eight session messages, listed first.
    let messages: Vec<&str> = dictionary
        .spec()
        .messages
        .iter()
        .map(|message| message.name.as_str())
        .collect();
    assert_eq!(messages.len(), 101);
    assert_eq!(messages[..3], ["Heartbeat", "TestRequest", "ResendRequest"]);
    assert_eq!(messages[8], "IOI");

    // The transport's MsgTypeGrp is empty, so Logon's group is the
    // application dictionary's.
    let msg_types = dictionary
        .messages()
        .find(|message| message.name() == "Logon")
        .unwrap()
        .members()
        .find_map(|member| match member {
            dictionary::Member::Group(group) => Some(group),
            dictionary::Member::Field { .. } => None,
        })
        .unwrap();
    assert_eq!(msg_types.count_field().name, "NoMsgTypes");
    assert_eq!(msg_types.component(), Some("MsgTypeGrp"));
    assert_eq!(msg_types.members().count(), 4);

    let field = |name: &str| {
        dictionary
            .fields()
            .iter()
            .find(|field| field.name == name)
            .unwrap()
    };
    // Ten fields are the transport's alone.
    assert_eq!(dictionary.fields().len(), 1100);
    assert_eq!(field("SessionStatus").tag, Tag(1409));
    // ApplVerID gains the two versions only the transport lists.
    let versions: Vec<&str> = field("ApplVerID")
        .values
        .iter()
        .map(|value| value.value.as_str())
        .collect();
    assert_eq!(versions, ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    // The application dictionary never lists Signature after its length.
    assert_eq!(field("Signature").length_tag, Some(Tag(93)));

    // Joining adds no warnings to the application dictionary's own.
    assert_eq!(parsed.warnings, alone.warnings);
}

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
            length_tag: None,
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
    assert_eq!(parties.component(), Some("Parties"));
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

    // Logon declares its one group inline, the only FIX44 group without a
    // component of its own.
    let msg_types = parsed
        .dictionary
        .messages()
        .find(|message| message.name() == "Logon")
        .unwrap()
        .members()
        .find_map(|member| match member {
            dictionary::Member::Group(group) => Some(group),
            dictionary::Member::Field { .. } => None,
        })
        .unwrap();
    assert_eq!(msg_types.count_field().name, "NoMsgTypes");
    assert_eq!(
        msg_types.declared_in(),
        &MemberContext::Message("Logon".to_owned())
    );
    assert_eq!(msg_types.component(), None);

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

    // The header ends with the NoHops group.
    let header: Vec<dictionary::Member> = parsed.dictionary.header().collect();
    assert_eq!(header.len(), 27);
    let Some(dictionary::Member::Group(hops)) = header.last() else {
        panic!("expected the header to end with a group");
    };
    assert_eq!(hops.count_field().name, "NoHops");
    assert_eq!(hops.declared_in(), &MemberContext::Header);

    // The trailer holds the signature fields and CheckSum.
    let trailer: Vec<&str> = parsed
        .dictionary
        .trailer()
        .map(|member| match member {
            dictionary::Member::Field { field, .. } => field.name.as_str(),
            dictionary::Member::Group(group) => panic!("unexpected group {group:?}"),
        })
        .collect();
    assert_eq!(trailer, ["SignatureLength", "Signature", "CheckSum"]);

    // Nothing is left unmodelled.
    assert!(parsed.warnings.is_empty());
}

#[test]
fn links_each_fix44_data_field_to_its_length_field() {
    let parsed = quickfix::parse(FIX44).unwrap();

    let mut links: Vec<(u32, u32)> = parsed
        .dictionary
        .fields()
        .iter()
        .filter_map(|field| field.length_tag.map(|length| (field.tag.0, length.0)))
        .collect();
    links.sort_unstable();

    assert_eq!(
        links,
        [
            (89, 93),
            (91, 90),
            (96, 95),
            (213, 212),
            (349, 348),
            (351, 350),
            (353, 352),
            (355, 354),
            (357, 356),
            (359, 358),
            (361, 360),
            (363, 362),
            (365, 364),
            (446, 445),
            (619, 618),
            (622, 621),
        ]
    );
}
