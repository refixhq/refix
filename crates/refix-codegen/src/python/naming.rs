use refix_dictionary::{EnumValue, Field, dictionary};

use crate::naming::{field_base_name, group_base_name};
use crate::python::Error;

pub(super) fn property_name(field: &Field) -> String {
    identifier(field_base_name(field))
}

pub(super) fn group_property_name(group: dictionary::Group<'_>) -> String {
    identifier(group_base_name(group))
}

pub(super) fn member_name(field: &Field, value: &EnumValue) -> Result<String, Error> {
    let mut name = value
        .description
        .trim_start_matches('_')
        .to_ascii_uppercase();

    // A name cannot start with a digit, so `5YR` becomes `N_5YR`.
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert_str(0, "N_");
    }
    let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let word_characters = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !starts_with_letter || !word_characters {
        return Err(Error::UnrepresentableValue {
            field: field.name.clone(),
            description: value.description.clone(),
        });
    }
    Ok(name)
}

/// Numbers a member an earlier value of its field is named like, so the
/// second `EURIBOR` becomes `EURIBOR_2`.
pub(super) fn numbered_member(name: &str, number: u32) -> String {
    format!("{name}_{number}")
}

/// Escapes a keyword with a trailing underscore, as PEP 8 suggests.
fn identifier(name: String) -> String {
    if KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// Python's lower-case keywords, the only ones a snake-case name can hit.
const KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

#[cfg(test)]
mod tests {
    use super::{member_name, property_name};
    use crate::python::Error;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn field(name: &str, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(1),
            data_type,
            values: vec![],
        }
    }

    fn member_of(description: &str) -> Result<String, Error> {
        let value = EnumValue {
            value: "1".to_owned(),
            description: description.to_owned(),
        };
        member_name(&field("OrdType", DataType::Char), &value)
    }

    #[test]
    fn property_names_are_snake_case() {
        assert_eq!(
            property_name(&field("ClOrdID", DataType::String)),
            "cl_ord_id"
        );
    }

    #[test]
    fn keyword_property_names_take_a_trailing_underscore() {
        assert_eq!(property_name(&field("Yield", DataType::String)), "yield_");
    }

    #[test]
    fn suffixed_raw_names_are_not_escaped() {
        let price = field("Yield", DataType::Other("PRICE".to_owned()));
        assert_eq!(property_name(&price), "yield_raw");
    }

    #[test]
    fn member_names_are_the_description_upper_cased() {
        assert_eq!(member_of("GOOD_TILL_CANCEL").unwrap(), "GOOD_TILL_CANCEL");
        assert_eq!(member_of("buy_minus").unwrap(), "BUY_MINUS");
    }

    #[test]
    fn a_digit_leading_description_takes_a_prefix() {
        assert_eq!(member_of("5YR").unwrap(), "N_5YR");
        assert_eq!(member_of("5_YR").unwrap(), "N_5_YR");
        assert_eq!(member_of("401K").unwrap(), "N_401K");
        assert_eq!(member_of("3").unwrap(), "N_3");
    }

    #[test]
    fn leading_underscores_are_dropped_before_the_prefix() {
        assert_eq!(
            member_of("_5_DAY_MOVING_AVERAGE").unwrap(),
            "N_5_DAY_MOVING_AVERAGE"
        );
        assert_eq!(member_of("_RESERVED_").unwrap(), "RESERVED_");
    }

    #[test]
    fn an_empty_description_is_an_error() {
        assert!(member_of("").is_err());
        assert!(member_of("_").is_err());
    }

    #[test]
    fn punctuation_in_a_description_is_an_error() {
        assert_eq!(
            member_of("W/AVG").unwrap_err(),
            Error::UnrepresentableValue {
                field: "OrdType".to_owned(),
                description: "W/AVG".to_owned(),
            }
        );
    }

    #[test]
    fn a_prefixed_name_still_passes_the_gate() {
        assert!(member_of("5/YR").is_err());
    }

    #[test]
    fn capitalized_keywords_are_fine() {
        assert_eq!(member_of("NONE").unwrap(), "NONE");
        assert_eq!(member_of("TRUE").unwrap(), "TRUE");
    }
}
