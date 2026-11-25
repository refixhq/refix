use refix_dictionary::{Dictionary, dictionary};

use super::groups::{KNOWN_TAGS, emit_group, emit_known_tags, emit_lazy_tags, known_tags_of};
use super::layout::{MAX_WIDTH, indent};
use super::messages::{Lifetime, emit_accessors};
use super::modules::SHARED;
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
    let walks_groups = envelope.with_groups.contains(&section);
    let from_scope = if walks_groups {
        "Self(scope, Self::envelope_tags().get())"
    } else {
        "Self(scope)"
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
            "/// The {} fields of a message.\n{}",
            section.member_name(),
            emit_view_struct(name, walks_groups)
        ),
        format!(
            "impl<'a> From<refix_message::Scope<'a>> for {name}<'a> {{\n    fn from(scope: refix_message::Scope<'a>) -> Self {{\n        {from_scope}\n    }}\n}}\n"
        ),
        format!("impl<'a> {name}<'a> {{\n{}}}\n", methods.join("\n")),
        emit_debug(name, &format!("self.0.display_known(&{KNOWN_TAGS})")),
    ];
    let groups = section_groups(section, dictionary);
    items.extend(groups.into_iter().map(|group| emit_group(group, names)));
    items.join("\n")
}

/// The groups a section declares itself.
pub(super) fn section_groups(
    section: Section,
    dictionary: &Dictionary,
) -> Vec<dictionary::Group<'_>> {
    let members = section.members(dictionary).into_iter();
    members
        .filter_map(|member| match member {
            flat::Member::Group(group) if *group.declared_in() == section.context() => Some(group),
            _ => None,
        })
        .collect()
}

/// A view's struct, which wraps a scope and, to walk groups, the known tags.
///
/// The rest of the generated code builds it from its fields.
pub(super) fn emit_view_struct(name: &str, keeps_known_tags: bool) -> String {
    let mut fields = vec![format!("{SHARED} refix_message::Scope<'a>")];
    if keeps_known_tags {
        fields.push(format!("{SHARED} KnownTags<'static>"));
    }
    let head = format!("#[derive(Clone, Copy)]\npub struct {name}<'a>(");
    let one_line = format!("pub struct {name}<'a>({});", fields.join(", "));
    if one_line.len() <= MAX_WIDTH {
        return format!("{head}{});\n", fields.join(", "));
    }
    let fields: String = fields
        .iter()
        .map(|field| format!("    {field},\n"))
        .collect();
    format!("{head}\n{fields});\n")
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

#[cfg(test)]
mod tests {
    use super::emit_view_struct;

    #[test]
    fn a_view_wraps_a_scope_that_other_files_can_set() {
        assert_eq!(
            emit_view_struct("Trailer", false),
            "#[derive(Clone, Copy)]\npub struct Trailer<'a>(pub(in super::super) refix_message::Scope<'a>);\n"
        );
    }

    #[test]
    fn a_view_that_walks_groups_keeps_the_known_tags() {
        assert_eq!(
            emit_view_struct("Header", true),
            "#[derive(Clone, Copy)]\npub struct Header<'a>(\n    pub(in super::super) refix_message::Scope<'a>,\n    pub(in super::super) KnownTags<'static>,\n);\n"
        );
    }

    #[test]
    fn a_long_name_puts_the_scope_on_its_own_line() {
        assert_eq!(
            emit_view_struct("UnderlyingStreamTerminationDateBusinessCenterGrp", false),
            "#[derive(Clone, Copy)]\npub struct UnderlyingStreamTerminationDateBusinessCenterGrp<'a>(\n    pub(in super::super) refix_message::Scope<'a>,\n);\n"
        );
    }
}
