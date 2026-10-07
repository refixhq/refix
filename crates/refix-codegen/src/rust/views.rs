use refix_dictionary::{Dictionary, dictionary};

use super::groups::{emit_group_accessor, emit_group_module, emit_known_tags};
use super::layout::{MAX_WIDTH, indent};
use super::messages::{Lifetime, emit_accessor};
use super::names::Names;
use crate::envelope::{Envelope, Section};

/// A section's view, and the module of the groups the section declares
/// itself.
///
/// A view whose section holds a group keeps the known tags to walk it with.
/// Built from a scope, it knows only the envelope's tags. A message passes
/// its own.
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
            "Self(scope, Self::KNOWN_TAGS)",
        )
    } else {
        ("(refix_message::Scope<'a>)", "Self(scope)")
    };

    let mut methods = Vec::new();
    if walks_groups {
        methods.push(indent(&emit_known_tags(&envelope.tags, MAX_WIDTH - 4), 1));
    }
    methods.push(
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    );
    for member in &members {
        match member {
            dictionary::Member::Field { field, .. } => {
                methods.push(emit_accessor(field, names, Lifetime::Scope));
            }
            dictionary::Member::Group(group) => {
                methods.push(emit_group_accessor(
                    *group,
                    names,
                    Lifetime::Scope,
                    "self.1",
                ));
            }
        }
    }

    let mut items = vec![
        format!("#[derive(Clone, Copy, Debug)]\npub struct {name}<'a>{fields};\n"),
        format!(
            "impl<'a> From<refix_message::Scope<'a>> for {name}<'a> {{\n    fn from(scope: refix_message::Scope<'a>) -> Self {{\n        {from_scope}\n    }}\n}}\n"
        ),
        format!("impl<'a> {name}<'a> {{\n{}}}\n", methods.join("\n")),
    ];
    let modules: Vec<String> = members
        .iter()
        .filter_map(|member| match member {
            dictionary::Member::Group(group) if *group.declared_in() == section.context() => {
                Some(emit_group_module(*group, names))
            }
            _ => None,
        })
        .collect();
    if !modules.is_empty() {
        items.push(format!(
            "pub mod {} {{\n{}}}\n",
            section.member_name(),
            indent(&modules.join("\n"), 1)
        ));
    }
    items.join("\n")
}

/// The message method returning a section's view. A view that walks groups
/// gets the message's known tags.
pub(super) fn emit_view_accessor(section: Section, envelope: &Envelope) -> String {
    let (name, type_name) = (section.member_name(), section.type_name());
    let arguments = if envelope.with_groups.contains(&section) {
        "self.0.scope(), Self::KNOWN_TAGS"
    } else {
        "self.0.scope()"
    };
    format!(
        "    pub fn {name}(&self) -> {type_name}<'_> {{\n        {type_name}({arguments})\n    }}\n"
    )
}
