use refix_dictionary::Field;

use super::names::Names;
use crate::literal::int_literal;

pub(super) fn emit_enum(field: &Field, names: &Names) -> String {
    let name = names.enum_type(field);
    let int_based = field.data_type.is_int_based();
    let variants = names.variants(field);
    let codes: Vec<String> = field
        .values
        .iter()
        .map(|value| {
            if int_based {
                int_literal(&value.value)
            } else {
                format!("{:?}", value.value)
            }
        })
        .collect();
    let (generics, payload) = if int_based {
        ("", "i64")
    } else {
        ("<'a>", "&'a str")
    };

    let declarations: String = variants
        .iter()
        .map(|variant| format!("    {variant},\n"))
        .collect();
    let from_arms: String = codes
        .iter()
        .zip(variants)
        .map(|(code, variant)| format!("            {code} => Self::{variant},\n"))
        .collect();
    let value_arms: String = codes
        .iter()
        .zip(variants)
        .map(|(code, variant)| format!("            Self::{variant} => {code},\n"))
        .collect();

    let mut items = vec![
        format!(
            "#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]\npub enum {name}{generics} {{\n{declarations}    Unrecognized({payload}),\n}}\n"
        ),
        format!(
            "impl{generics} {name}{generics} {{\n    pub fn from_value(value: {payload}) -> Self {{\n        match value {{\n{from_arms}            unrecognized => Self::Unrecognized(unrecognized),\n        }}\n    }}\n\n    pub fn value(self) -> {payload} {{\n        match self {{\n{value_arms}            Self::Unrecognized(value) => value,\n        }}\n    }}\n}}\n"
        ),
    ];
    if !int_based {
        items.push(format!(
            "impl<'a> From<&'a str> for {name}<'a> {{\n    fn from(value: &'a str) -> Self {{\n        Self::from_value(value)\n    }}\n}}\n"
        ));
    }

    items.join("\n")
}

#[cfg(test)]
mod tests {
    use super::emit_enum;
    use crate::rust::names::Names;
    use crate::test_utils::message_with;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn emitted(field: Field) -> String {
        let dictionary = message_with(vec![field.clone()]);
        emit_enum(&field, &Names::new(&dictionary).unwrap())
    }

    fn enum_field(name: &str, data_type: DataType, values: &[(&str, &str)]) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(1),
            data_type,
            values: values
                .iter()
                .map(|(value, description)| EnumValue {
                    value: (*value).to_owned(),
                    description: (*description).to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn an_int_based_field_gets_an_int_coded_enum() {
        let field = enum_field(
            "PartyRole",
            DataType::Int,
            &[("1", "EXECUTING_FIRM"), ("3", "CLIENT_ID")],
        );

        let expected = "\
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PartyRole {
    ExecutingFirm,
    ClientId,
    Unrecognized(i64),
}

impl PartyRole {
    pub fn from_value(value: i64) -> Self {
        match value {
            1 => Self::ExecutingFirm,
            3 => Self::ClientId,
            unrecognized => Self::Unrecognized(unrecognized),
        }
    }

    pub fn value(self) -> i64 {
        match self {
            Self::ExecutingFirm => 1,
            Self::ClientId => 3,
            Self::Unrecognized(value) => value,
        }
    }
}
";
        assert_eq!(emitted(field), expected);
    }

    #[test]
    fn int_codes_are_written_canonically() {
        let field = enum_field(
            "Code",
            DataType::Int,
            &[("007", "SEVEN"), ("-02", "MINUS_TWO"), ("000", "ZERO")],
        );

        let code = emitted(field);

        assert!(code.contains("            7 => Self::Seven,\n"));
        assert!(code.contains("            -2 => Self::MinusTwo,\n"));
        assert!(code.contains("            0 => Self::Zero,\n"));
    }

    #[test]
    fn string_codes_are_escaped() {
        let field = enum_field("Code", DataType::String, &[("a\"b\\", "ODD")]);

        let code = emitted(field);

        assert!(code.contains("            \"a\\\"b\\\\\" => Self::Odd,\n"));
    }
}
