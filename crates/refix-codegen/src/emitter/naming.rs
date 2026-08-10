use refix_dictionary::{DataType, EnumValue, Field};

use crate::emitter::Error;
use crate::{pascal_case, snake_case};

pub(super) fn method_name(field: &Field) -> Result<String, Error> {
    let mut name = snake_case(&field.name);
    if field.values.is_empty() && matches!(field.data_type, DataType::Other(_)) {
        name.push_str("_raw");
    }
    if matches!(name.as_str(), "self" | "super" | "crate") {
        return Err(Error::UnrepresentableName {
            field: field.name.clone(),
        });
    }
    if RESERVED_WORDS.contains(&name.as_str()) {
        return Ok(format!("r#{name}"));
    }
    Ok(name)
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
    use super::{method_name, variant_name};
    use crate::emitter::Error;
    use refix_dictionary::{DataType, EnumValue, Field};

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag,
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
