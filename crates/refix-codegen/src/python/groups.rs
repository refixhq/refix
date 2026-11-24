use refix_dictionary::{MemberContext, dictionary};

use super::declarations::emit_declarations;
use super::layout::indent;
use super::names::Names;
use crate::flat;

/// The class of a group's instances.
///
/// A group declared directly in its instances has its class in the body,
/// where the declaration reading it can name it.
pub(super) fn emit_group_class(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut items: Vec<String> = nested_groups(group)
        .map(|nested| indent(&emit_group_class(nested, names), 1))
        .collect();
    items.push(emit_declarations(group.members(), names));
    format!(
        "class {}(fields.GroupInstance):\n{}",
        names.group_class(group),
        items.join("\n")
    )
}

/// The groups declared directly in the instances of `group`.
fn nested_groups<'a>(group: dictionary::Group<'a>) -> impl Iterator<Item = dictionary::Group<'a>> {
    flat::members(group.members()).filter_map(|member| match member {
        flat::Member::Group(nested) if is_declared_in_a_group(nested) => Some(nested),
        _ => None,
    })
}

fn is_declared_in_a_group(group: dictionary::Group<'_>) -> bool {
    matches!(group.declared_in(), MemberContext::Group { .. })
}
