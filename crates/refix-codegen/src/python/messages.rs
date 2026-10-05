use refix_dictionary::{DataType, Field, dictionary};

use super::enums::str_literal;
use super::naming::property_name;

pub(super) fn emit_message(message: dictionary::Message<'_>) -> String {
    let mut members = vec![
        format!(
            "    MSG_TYPE: ClassVar[bytes] = b{}\n",
            str_literal(message.msg_type())
        ),
        "    def __init__(self, raw: RawMessage) -> None:\n        self._raw = raw\n".to_owned(),
        "    @property\n    def raw(self) -> RawMessage:\n        return self._raw\n".to_owned(),
    ];
    members.extend(message.members().filter_map(|member| match member {
        dictionary::Member::Field { field, .. } => Some(emit_property(field)),
        dictionary::Member::Group(_) => None,
    }));
    format!("class {}:\n{}", message.name(), members.join("\n"))
}

pub(super) fn emit_property(field: &Field) -> String {
    let name = property_name(field);
    let tag = field.tag;
    let type_name = &field.name;
    let (return_type, body) = if field.values.is_empty() {
        match field.data_type {
            DataType::String => (
                "str | None".to_owned(),
                format!("return self._raw.get_str({tag})"),
            ),
            DataType::Int => (
                "int | None".to_owned(),
                format!("return self._raw.get_int({tag})"),
            ),
            _ => (
                "bytes | None".to_owned(),
                format!("return self._raw.get({tag})"),
            ),
        }
    } else if field.data_type.is_multiple_value() {
        (
            format!("tuple[{type_name} | Unrecognized[str], ...] | None"),
            format!(
                "values = self._raw.get_multiple_values({tag})\n        return None if values is None else tuple(from_value({type_name}, value) for value in values)"
            ),
        )
    } else {
        let (conversion, value_type) = if field.data_type.is_int_based() {
            ("get_int", "int")
        } else {
            ("get_str", "str")
        };
        (
            format!("{type_name} | Unrecognized[{value_type}] | None"),
            format!(
                "value = self._raw.{conversion}({tag})\n        return None if value is None else from_value({type_name}, value)"
            ),
        )
    };
    format!("    @cached_property\n    def {name}(self) -> {return_type}:\n        {body}\n")
}
