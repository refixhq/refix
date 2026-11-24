use refix_dictionary::Dictionary;

use super::declarations::emit_declarations;
use super::groups::emit_group_class;
use super::layout::indent;
use super::names::Names;
use crate::envelope::Section;
use crate::flat;

/// A section's component class, with the classes of the groups the section
/// declares itself nested in it.
pub(super) fn emit_view(section: Section, dictionary: &Dictionary, names: &Names) -> String {
    let members = section.members(dictionary);
    let mut items: Vec<String> = members
        .iter()
        .filter_map(|member| match member {
            flat::Member::Group(group) if *group.declared_in() == section.context() => {
                Some(indent(&emit_group_class(*group, names), 1))
            }
            _ => None,
        })
        .collect();
    let listed = section.listed(dictionary).into_iter();
    items.push(emit_declarations(listed, names));
    format!(
        "class {}(fields.Component):\n{}",
        section.type_name(),
        items.join("\n")
    )
}

/// The declaration in a message reading a section as a view.
pub(super) fn emit_view_declaration(section: Section) -> String {
    let (name, type_name) = (section.member_name(), section.type_name());
    format!("    {name} = fields.View({type_name})\n")
}
