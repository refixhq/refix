use refix_dictionary::{DataType, EnumValue, Field};

use crate::emitter::Error;
use crate::{pascal_case, snake_case};

pub(super) fn method_name(field: &Field) -> Result<String, Error> {
    let mut name = snake_case(&field.name);
    if let DataType::Other(_) = field.data_type {
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
    use super::variant_name;
    use crate::emitter::Error;
    use refix_dictionary::{DataType, EnumValue, Field};

    fn variant_of(description: &str) -> Result<String, Error> {
        let field = Field {
            name: "OrdType".to_owned(),
            tag: 40,
            data_type: DataType::Other("CHAR".to_owned()),
            values: vec![],
        };
        let value = EnumValue {
            value: "1".to_owned(),
            description: description.to_owned(),
        };
        variant_name(&field, &value)
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
}
