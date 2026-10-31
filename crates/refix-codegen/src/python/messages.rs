use refix_dictionary::{Field, MemberContext, Tag, dictionary};

use super::enums::str_literal;
use super::groups::{emit_group_class, emit_group_property, tags};
use super::layout::{MAX_WIDTH, indent, tuple_lines};
use super::names::Names;
use super::views::emit_view_property;
use crate::conversion::Conversion;
use crate::envelope::Envelope;
use crate::groups::message_known_tags;

/// A message's class. Its known tags include the `envelope`'s, and it
/// reaches each view the envelope has.
pub(super) fn emit_message(
    message: dictionary::Message<'_>,
    envelope: &Envelope,
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
                &tags(&message_known_tags(message, &envelope.tags)),
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
    for section in &envelope.sections {
        members.push(emit_view_property(*section, name, envelope));
    }
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
    let (return_type, body) = match Conversion::of(field) {
        Conversion::Str => (
            "str | None".to_owned(),
            format!("return {receiver}.get_str({tag})"),
        ),
        Conversion::Int => (
            "int | None".to_owned(),
            format!("return {receiver}.get_int({tag})"),
        ),
        Conversion::StrEnum => {
            enum_property(names.enum_class(field), receiver, tag, "get_str", "str")
        }
        Conversion::IntEnum => {
            enum_property(names.enum_class(field), receiver, tag, "get_int", "int")
        }
        Conversion::MultipleValues => {
            let type_name = names.enum_class(field);
            (
                format!("tuple[{type_name} | Unrecognized[str], ...] | None"),
                format!(
                    "values = {receiver}.get_multiple_values({tag})\n        return None if values is None else tuple(from_value({type_name}, value) for value in values)"
                ),
            )
        }
        Conversion::Raw => (
            "bytes | None".to_owned(),
            format!("return {receiver}.get({tag})"),
        ),
    };
    format!("    @cached_property\n    def {name}(self) -> {return_type}:\n        {body}\n")
}

/// The return type and body of a property reading `tag` with `getter` and
/// classifying it into the `type_name` enum.
fn enum_property(
    type_name: &str,
    receiver: &str,
    tag: Tag,
    getter: &str,
    value_type: &str,
) -> (String, String) {
    (
        format!("{type_name} | Unrecognized[{value_type}] | None"),
        format!(
            "value = {receiver}.{getter}({tag})\n        return None if value is None else from_value({type_name}, value)"
        ),
    )
}
