use refix_dictionary::{DataType, EnumValue, Field, dictionary};

use crate::emitter::Error;
use crate::{pascal_case, snake_case};

pub(super) fn method_name(field: &Field) -> Result<String, Error> {
    let mut name = snake_case(&field.name);
    if field.values.is_empty() && matches!(field.data_type, DataType::Other(_)) {
        name.push_str("_raw");
    }
    identifier(name).ok_or_else(|| Error::UnrepresentableName {
        field: field.name.clone(),
    })
}

/// The accessor and module name of a group.
///
/// The component it makes up, else its count field's name without the `No` prefix,
/// else its count field's name.
pub(super) fn group_name(group: dictionary::Group<'_>) -> Result<String, Error> {
    let count_field = group.count_field().name.as_str();
    let name = group
        .component()
        .or_else(|| without_no_prefix(count_field))
        .unwrap_or(count_field);
    identifier(snake_case(name)).ok_or_else(|| Error::UnrepresentableGroupName {
        context: group.declared_in().group(count_field),
    })
}

pub(super) fn variant_name(field: &Field, value: &EnumValue) -> Result<String, Error> {
    let name = pascal_case(&value.description);
    let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let alphanumeric = name.chars().all(|c| c.is_ascii_alphanumeric());
    if !starts_with_letter || !alphanumeric || name == "Self" {
        return Err(Error::UnrepresentableValue {
            field: field.name.clone(),
            description: value.description.clone(),
        });
    }
    Ok(name)
}

/// Strips a `No` that starts a word: `NoPartyIDs`, but not `Notional`.
fn without_no_prefix(name: &str) -> Option<&str> {
    name.strip_prefix("No")
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
}

/// Escapes a keyword as a raw identifier, or `None` for a keyword that
/// cannot be escaped.
fn identifier(name: String) -> Option<String> {
    if matches!(name.as_str(), "self" | "super" | "crate") {
        return None;
    }
    if RESERVED_WORDS.contains(&name.as_str()) {
        return Some(format!("r#{name}"));
    }
    Some(name)
}

/// Rust's strict and reserved keywords.
const RESERVED_WORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

#[cfg(test)]
mod tests {
    use super::{group_name, method_name, variant_name};
    use crate::emitter::Error;
    use refix_dictionary::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Group, Member,
        MemberContext, Message, Protocol, Spec, Tag, Version, dictionary,
    };

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            values: vec![],
        }
    }

    fn variant_of(description: &str) -> Result<String, Error> {
        let value = EnumValue {
            value: "1".to_owned(),
            description: description.to_owned(),
        };
        variant_name(
            &field("OrdType", 40, DataType::Other("CHAR".to_owned())),
            &value,
        )
    }

    /// Names a group counted by `count_field`, declared inline in a message
    /// or as the sole member of `component`.
    fn group_named(component: Option<&str>, count_field: &str) -> Result<String, Error> {
        let group = Member::Group(Group {
            count_tag: Tag(453),
            is_required: false,
            members: vec![Member::Field(FieldRef {
                tag: Tag(448),
                is_required: false,
            })],
        });
        let (members, components) = match component {
            Some(name) => (
                vec![Member::Component(ComponentRef {
                    name: name.to_owned(),
                    is_required: false,
                })],
                vec![Component {
                    name: name.to_owned(),
                    members: vec![group],
                }],
            ),
            None => (vec![group], vec![]),
        };
        let dictionary = Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            messages: vec![Message {
                name: "NewOrderSingle".to_owned(),
                msg_type: "D".to_owned(),
                members,
                category: Category::App,
            }],
            fields: vec![
                field(count_field, 453, DataType::Other("NUMINGROUP".to_owned())),
                field("PartyID", 448, DataType::String),
            ],
            components,
        }
        .resolve()
        .unwrap();
        let message = dictionary.messages().next().unwrap();
        match message.members().next() {
            Some(dictionary::Member::Group(group)) => group_name(group),
            other => panic!("expected a group, found {other:?}"),
        }
    }

    #[test]
    fn method_names_are_snake_case() {
        let name = method_name(&field("ClOrdID", 11, DataType::String));
        assert_eq!(name.unwrap(), "cl_ord_id");
    }

    #[test]
    fn keyword_method_names_are_escaped() {
        let name = method_name(&field("Yield", 236, DataType::String));
        assert_eq!(name.unwrap(), "r#yield");
    }

    #[test]
    fn suffixed_raw_names_are_not_escaped() {
        let name = method_name(&field("Yield", 236, DataType::Other("PRICE".to_owned())));
        assert_eq!(name.unwrap(), "yield_raw");
    }

    #[test]
    fn an_unescapable_name_is_an_error() {
        assert_eq!(
            method_name(&field("Self", 9000, DataType::String)).unwrap_err(),
            Error::UnrepresentableName {
                field: "Self".to_owned(),
            }
        );
    }

    #[test]
    fn a_suffix_makes_an_unescapable_name_legal() {
        let name = method_name(&field("Self", 9000, DataType::Other("DATA".to_owned())));
        assert_eq!(name.unwrap(), "self_raw");
    }

    #[test]
    fn a_group_takes_the_name_of_its_component() {
        let name = group_named(Some("Parties"), "NoPartyIDs");
        assert_eq!(name.unwrap(), "parties");
    }

    #[test]
    fn an_inline_group_drops_the_no_prefix() {
        let name = group_named(None, "NoMsgTypes");
        assert_eq!(name.unwrap(), "msg_types");
    }

    #[test]
    fn a_no_that_does_not_start_a_word_is_kept() {
        let name = group_named(None, "Nominees");
        assert_eq!(name.unwrap(), "nominees");
    }

    #[test]
    fn a_count_field_without_the_prefix_is_used_as_is() {
        let name = group_named(None, "LegCount");
        assert_eq!(name.unwrap(), "leg_count");
    }

    #[test]
    fn keyword_group_names_are_escaped() {
        let name = group_named(Some("Type"), "NoTypes");
        assert_eq!(name.unwrap(), "r#type");
    }

    #[test]
    fn an_unescapable_group_name_is_an_error() {
        assert_eq!(
            group_named(None, "NoSelf").unwrap_err(),
            Error::UnrepresentableGroupName {
                context: MemberContext::Message("NewOrderSingle".to_owned()).group("NoSelf"),
            }
        );
    }

    #[test]
    fn converts_a_description_to_pascal_case() {
        assert_eq!(variant_of("GOOD_TILL_CANCEL").unwrap(), "GoodTillCancel");
    }

    #[test]
    fn a_digit_leading_description_is_an_error() {
        assert_eq!(
            variant_of("5DAY").unwrap_err(),
            Error::UnrepresentableValue {
                field: "OrdType".to_owned(),
                description: "5DAY".to_owned(),
            }
        );
    }

    #[test]
    fn an_empty_description_is_an_error() {
        assert!(variant_of("").is_err());
    }

    #[test]
    fn punctuation_in_a_description_is_an_error() {
        assert!(variant_of("W/AVG").is_err());
    }

    #[test]
    fn a_non_ascii_description_is_an_error() {
        assert!(variant_of("CAF\u{c9}").is_err());
    }

    #[test]
    fn self_is_an_error() {
        assert!(variant_of("SELF").is_err());
    }

    #[test]
    fn a_description_containing_self_is_fine() {
        assert_eq!(variant_of("SELF_TRADE").unwrap(), "SelfTrade");
    }

    #[test]
    fn capitalized_keywords_are_fine() {
        assert_eq!(variant_of("TRUE").unwrap(), "True");
        assert_eq!(variant_of("SUPER").unwrap(), "Super");
    }

    #[test]
    fn an_enum_field_takes_the_plain_name() {
        let mut enum_field = field("OrdType", 40, DataType::Other("CHAR".to_owned()));
        enum_field.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "MARKET".to_owned(),
        }];
        assert_eq!(method_name(&enum_field).unwrap(), "ord_type");
    }
}
