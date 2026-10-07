use refix_dictionary::Field;

use super::names::Names;
use crate::literal::int_literal;

pub(super) fn emit_enum(field: &Field, names: &Names) -> String {
    let int_based = field.data_type.is_int_based();
    let base = if int_based { "IntEnum" } else { "StrEnum" };
    let members: String = field
        .values
        .iter()
        .zip(names.enum_members(field))
        .map(|(value, member)| {
            let code = if int_based {
                int_literal(&value.value)
            } else {
                str_literal(&value.value)
            };
            format!("    {member} = {code}\n")
        })
        .collect();
    format!("class {}(enum.{base}):\n{members}", names.enum_class(field))
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
    use crate::python::names::Names;
    use crate::test_utils::message_with;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn emitted(field: Field) -> String {
        let dictionary = message_with(vec![field.clone()]);
        emit_enum(&field, &Names::new(&dictionary, &mut Vec::new()).unwrap())
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
    fn an_int_based_field_gets_an_int_enum() {
        let field = enum_field(
            "PartyRole",
            DataType::Int,
            &[("1", "EXECUTING_FIRM"), ("03", "CLIENT_ID")],
        );

        assert_eq!(
            emitted(field),
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
            emitted(field),
            "class OrdType(enum.StrEnum):\n    MARKET = \"1\"\n    LIMIT = \"2\"\n"
        );
    }

    #[test]
    fn string_literals_are_escaped() {
        assert_eq!(str_literal("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(str_literal("\u{1}\u{e9}"), r#""\u0001\u00e9""#);
    }
}
