use refix_dictionary::{EnumValue, Field, dictionary};

use crate::naming::{field_base_name, group_base_name};
use crate::rust::Error;
use crate::{pascal_case, snake_case};

pub(super) fn method_name(field: &Field) -> Result<String, Error> {
    identifier(field_base_name(field)).ok_or_else(|| Error::UnrepresentableName {
        field: field.name.clone(),
    })
}

/// The accessor and module name of a group, escaped for Rust.
pub(super) fn group_name(group: dictionary::Group<'_>) -> Result<String, Error> {
    identifier(group_base_name(group)).ok_or_else(|| Error::UnrepresentableGroupName {
        context: group.declared_in().group(&group.count_field().name),
    })
}

pub(super) fn variant_name(field: &Field, value: &EnumValue) -> Result<String, Error> {
    let mut name = pascal_case(&value.description);

    // A name cannot start with a digit, so `5YR` becomes `N5yr`.
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, 'N');
    }

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

/// The name of the module holding a message's own groups.
pub(super) fn message_module_name(message: &str) -> Result<String, Error> {
    identifier(snake_case(message)).ok_or_else(|| Error::UnrepresentableMessageName {
        message: message.to_owned(),
    })
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
    use super::{group_name, message_module_name, method_name, variant_name};
    use crate::rust::Error;
    use crate::test_utils::with_group;
    use refix_dictionary::{DataType, EnumValue, Field, MemberContext, Tag};

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
        variant_name(&field("OrdType", 40, DataType::Char), &value)
    }

    fn group_named(component: Option<&str>, count_field: &str) -> Result<String, Error> {
        with_group(component, count_field, group_name)
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
    fn message_module_names_are_snake_case() {
        let name = message_module_name("NewOrderSingle");
        assert_eq!(name.unwrap(), "new_order_single");
    }

    #[test]
    fn an_unescapable_message_module_name_is_an_error() {
        assert_eq!(
            message_module_name("Self").unwrap_err(),
            Error::UnrepresentableMessageName {
                message: "Self".to_owned(),
            }
        );
    }

    #[test]
    fn converts_a_description_to_pascal_case() {
        assert_eq!(variant_of("GOOD_TILL_CANCEL").unwrap(), "GoodTillCancel");
    }

    #[test]
    fn a_digit_leading_description_takes_a_prefix() {
        assert_eq!(variant_of("5YR").unwrap(), "N5yr");
        assert_eq!(variant_of("5_YR").unwrap(), "N5Yr");
        assert_eq!(variant_of("401K").unwrap(), "N401k");
        assert_eq!(variant_of("3").unwrap(), "N3");
    }

    #[test]
    fn leading_underscores_are_dropped_before_the_prefix() {
        assert_eq!(
            variant_of("_5_DAY_MOVING_AVERAGE").unwrap(),
            "N5DayMovingAverage"
        );
    }

    #[test]
    fn an_empty_description_is_an_error() {
        assert!(variant_of("").is_err());
    }

    #[test]
    fn punctuation_in_a_description_is_an_error() {
        assert_eq!(
            variant_of("W/AVG").unwrap_err(),
            Error::UnrepresentableValue {
                field: "OrdType".to_owned(),
                description: "W/AVG".to_owned(),
            }
        );
    }

    #[test]
    fn a_prefixed_name_still_passes_the_gate() {
        assert!(variant_of("5/YR").is_err());
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
}
