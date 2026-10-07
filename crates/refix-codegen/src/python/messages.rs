use refix_dictionary::{DataType, Field, MemberContext, Tag, dictionary};

use super::enums::str_literal;
use super::groups::{emit_group_class, emit_group_property, tags};
use super::layout::{MAX_WIDTH, indent, tuple_lines};
use super::names::Names;
use crate::groups::message_known_tags;

/// A message's class. Its known tags include the `envelope`'s.
pub(super) fn emit_message(
    message: dictionary::Message<'_>,
    envelope: &[Tag],
    names: &Names,
) -> String {
    let name = message.name();
    let mut members = vec![
        format!(
            "    MSG_TYPE: ClassVar[bytes] = b{}\n",
            str_literal(message.msg_type())
        ),
        indent(
            &tuple_lines(
                "KNOWN_TAGS: ClassVar[KnownTags] = KnownTags(",
                &tags(&message_known_tags(message, envelope)),
                ")",
                MAX_WIDTH - 4,
            ),
            1,
        ),
    ];
    members.extend(message.members().filter_map(|member| match member {
        dictionary::Member::Group(group)
            if matches!(group.declared_in(), MemberContext::Message(_)) =>
        {
            Some(indent(&emit_group_class(group, 1, names), 1))
        }
        _ => None,
    }));
    members.push(
        "    def __init__(self, raw: RawMessage) -> None:\n        self._raw = raw\n".to_owned(),
    );
    members.push(
        "    @property\n    def raw(self) -> RawMessage:\n        return self._raw\n".to_owned(),
    );
    let known_tags = format!("{name}.KNOWN_TAGS");
    members.extend(message.members().map(|member| match member {
        dictionary::Member::Field { field, .. } => emit_property(field, names, "self._raw"),
        dictionary::Member::Group(group) => {
            emit_group_property(group, names, "self._raw", &known_tags)
        }
    }));
    format!("class {name}:\n{}", members.join("\n"))
}

/// The property reading `field` from `receiver`, the message or the instance's scope.
pub(super) fn emit_property(field: &Field, names: &Names, receiver: &str) -> String {
    let name = names.property(field);
    let tag = field.tag;
    let (return_type, body) = if field.values.is_empty() {
        match field.data_type {
            DataType::String => (
                "str | None".to_owned(),
                format!("return {receiver}.get_str({tag})"),
            ),
            DataType::Int => (
                "int | None".to_owned(),
                format!("return {receiver}.get_int({tag})"),
            ),
            _ => (
                "bytes | None".to_owned(),
                format!("return {receiver}.get({tag})"),
            ),
        }
    } else if field.data_type.is_multiple_value() {
        let type_name = names.enum_class(field);
        (
            format!("tuple[{type_name} | Unrecognized[str], ...] | None"),
            format!(
                "values = {receiver}.get_multiple_values({tag})\n        return None if values is None else tuple(from_value({type_name}, value) for value in values)"
            ),
        )
    } else {
        let type_name = names.enum_class(field);
        let (conversion, value_type) = if field.data_type.is_int_based() {
            ("get_int", "int")
        } else {
            ("get_str", "str")
        };
        (
            format!("{type_name} | Unrecognized[{value_type}] | None"),
            format!(
                "value = {receiver}.{conversion}({tag})\n        return None if value is None else from_value({type_name}, value)"
            ),
        )
    };
    format!("    @cached_property\n    def {name}(self) -> {return_type}:\n        {body}\n")
}
