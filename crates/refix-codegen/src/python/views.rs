use refix_dictionary::{Dictionary, dictionary};

use super::groups::{emit_group_class, emit_group_property, tags};
use super::layout::{MAX_WIDTH, indent, tuple_lines};
use super::messages::emit_property;
use super::names::Names;
use crate::envelope::{Envelope, Section};

/// A section's view class, with the classes of the groups the section
/// declares itself nested in it.
///
/// A view whose section holds a group keeps the known tags to walk it with.
/// Built from a raw message alone, it knows only the envelope's tags. A
/// message passes its own.
pub(super) fn emit_view(
    section: Section,
    dictionary: &Dictionary,
    envelope: &Envelope,
    names: &Names,
) -> String {
    let members = section.members(dictionary);
    let walks_groups = envelope.with_groups.contains(&section);

    let mut items = Vec::new();
    if walks_groups {
        items.push(indent(
            &tuple_lines(
                "KNOWN_TAGS: ClassVar[KnownTags] = KnownTags(",
                &tags(&envelope.tags),
                ")",
                MAX_WIDTH - 4,
            ),
            1,
        ));
    }
    items.extend(members.iter().filter_map(|member| match member {
        dictionary::Member::Group(group) if *group.declared_in() == section.context() => {
            Some(indent(&emit_group_class(*group, 1, names), 1))
        }
        _ => None,
    }));
    items.push(if walks_groups {
        "    def __init__(self, raw: RawMessage, known_tags: KnownTags = KNOWN_TAGS) -> None:\n        self._raw = raw\n        self._known_tags = known_tags\n".to_owned()
    } else {
        "    def __init__(self, raw: RawMessage) -> None:\n        self._raw = raw\n".to_owned()
    });
    items.push(
        "    @property\n    def raw(self) -> RawMessage:\n        return self._raw\n".to_owned(),
    );
    items.extend(members.iter().map(|member| match member {
        dictionary::Member::Field { field, .. } => emit_property(field, names, "self._raw"),
        dictionary::Member::Group(group) => {
            emit_group_property(*group, names, "self._raw", "self._known_tags")
        }
    }));
    format!("class {}:\n{}", section.type_name(), items.join("\n"))
}

/// The message property returning a section's view. A view that walks
/// groups gets the message's known tags.
pub(super) fn emit_view_property(section: Section, message: &str, envelope: &Envelope) -> String {
    let (name, type_name) = (section.member_name(), section.type_name());
    let arguments = if envelope.with_groups.contains(&section) {
        format!("self._raw, {message}.KNOWN_TAGS")
    } else {
        "self._raw".to_owned()
    };
    format!(
        "    @cached_property\n    def {name}(self) -> {type_name}:\n        return {type_name}({arguments})\n"
    )
}
