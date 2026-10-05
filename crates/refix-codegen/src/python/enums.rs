use refix_dictionary::Field;

use super::Error;
use super::naming::member_name;
use crate::literal::int_literal;

pub(super) fn emit_enum(field: &Field) -> Result<String, Error> {
    let int_based = field.data_type.is_int_based();
    let base = if int_based { "IntEnum" } else { "StrEnum" };
    let members = field
        .values
        .iter()
        .map(|value| {
            let code = if int_based {
                int_literal(&value.value)
            } else {
                str_literal(&value.value)
            };
            Ok(format!("    {} = {code}\n", member_name(field, value)?))
        })
        .collect::<Result<String, Error>>()?;
    Ok(format!("class {}(enum.{base}):\n{members}", field.name))
}

/// A Python string literal of `value`, escaping quotes, backslashes and
/// anything outside printable ASCII.
pub(super) fn str_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for c in value.chars() {
        match c {
            '"' | '\\' => {
                literal.push('\\');
                literal.push(c);
            }
            ' '..='~' => literal.push(c),
            c if u32::from(c) <= 0xFFFF => literal.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => literal.push_str(&format!("\\U{:08x}", u32::from(c))),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::{emit_enum, str_literal};
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

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
    fn an_int_based_field_gets_an_int_enum() {
        let field = enum_field(
            "PartyRole",
            DataType::Int,
            &[("1", "EXECUTING_FIRM"), ("03", "CLIENT_ID")],
        );

        assert_eq!(
            emit_enum(&field).unwrap(),
            "class PartyRole(enum.IntEnum):\n    EXECUTING_FIRM = 1\n    CLIENT_ID = 3\n"
        );
    }

    #[test]
    fn other_fields_get_a_str_enum() {
        let field = enum_field(
            "OrdType",
            DataType::Char,
            &[("1", "MARKET"), ("2", "LIMIT")],
        );

        assert_eq!(
            emit_enum(&field).unwrap(),
            "class OrdType(enum.StrEnum):\n    MARKET = \"1\"\n    LIMIT = \"2\"\n"
        );
    }

    #[test]
    fn string_literals_are_escaped() {
        assert_eq!(str_literal("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(str_literal("\u{1}\u{e9}"), r#""\u0001\u00e9""#);
    }
}
