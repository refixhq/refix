use refix_dictionary::Dictionary;

use super::groups::{KNOWN_TAGS, emit_group, emit_known_tags, emit_lazy_tags, known_tags_of};
use super::layout::indent;
use super::messages::{Lifetime, emit_accessors};
use super::names::Names;
use super::printing::emit_debug;
use crate::envelope::{Envelope, Section};
use crate::flat;

/// A section's view, then the types of the groups the section declares
/// itself.
///
/// A view whose section holds a group keeps the known tags to walk it with.
/// Built from a scope, it knows only the envelope's tags. A message passes
/// its own. `Debug` prints the section's own fields.
pub(super) fn emit_view(
    section: Section,
    dictionary: &Dictionary,
    envelope: &Envelope,
    names: &Names,
) -> String {
    let name = section.type_name();
    let members = section.members(dictionary);
    let walks_groups = envelope.with_groups.contains(&section);
    let (fields, from_scope) = if walks_groups {
        (
            "(refix_message::Scope<'a>, KnownTags<'static>)",
            "Self(scope, Self::envelope_tags().get())",
        )
    } else {
        ("(refix_message::Scope<'a>)", "Self(scope)")
    };

    let mut methods = Vec::new();
    if walks_groups {
        let sections: Vec<String> = envelope
            .sections
            .iter()
            .map(|section| known_tags_of(section.type_name()))
            .collect();
        methods.push(indent(&emit_lazy_tags("envelope_tags", &[], &sections), 1));
    }
    methods.push(indent(
        &emit_known_tags(section.listed(dictionary).into_iter(), &[], names),
        1,
    ));
    methods.push(
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    );
    methods.extend(emit_accessors(
        section.listed(dictionary).into_iter(),
        names,
        Lifetime::Scope,
        "self.1",
    ));

    let mut items = vec![
        format!(
            "/// The {} fields of a message.\n#[derive(Clone, Copy)]\npub struct {name}<'a>{fields};\n",
            section.member_name()
        ),
        format!(
            "impl<'a> From<refix_message::Scope<'a>> for {name}<'a> {{\n    fn from(scope: refix_message::Scope<'a>) -> Self {{\n        {from_scope}\n    }}\n}}\n"
        ),
        format!("impl<'a> {name}<'a> {{\n{}}}\n", methods.join("\n")),
        emit_debug(name, &format!("self.0.display_known(&{KNOWN_TAGS})")),
    ];
    items.extend(members.iter().filter_map(|member| match member {
        flat::Member::Group(group) if *group.declared_in() == section.context() => {
            Some(emit_group(*group, names))
        }
        _ => None,
    }));
    items.join("\n")
}

/// The message method returning a section's view. A view that walks groups
/// gets the message's known tags.
pub(super) fn emit_view_accessor(section: Section, envelope: &Envelope) -> String {
    let (name, type_name) = (section.member_name(), section.type_name());
    let arguments = if envelope.with_groups.contains(&section) {
        format!("self.0.scope(), {KNOWN_TAGS}")
    } else {
        "self.0.scope()".to_owned()
    };
    format!(
        "    pub fn {name}(&self) -> {type_name}<'_> {{\n        {type_name}({arguments})\n    }}\n"
    )
}
