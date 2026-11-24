use refix_dictionary::{Dictionary, MemberContext, dictionary};

use super::declarations::emit_declarations;
use super::layout::indent;
use super::names::Names;
use crate::flat;

/// The groups declared in components, each once, innermost first, from the
/// header, messages and trailer.
///
/// A class is defined after the classes its declarations name. Every other
/// group's class sits where the group is declared.
pub(super) fn shared_groups(dictionary: &Dictionary) -> Vec<dictionary::Group<'_>> {
    let mut groups = Vec::new();
    let mut seen = Vec::new();
    collect_groups(flat::members(dictionary.header()), &mut groups, &mut seen);
    for message in dictionary.messages() {
        collect_groups(flat::members(message.members()), &mut groups, &mut seen);
    }
    collect_groups(flat::members(dictionary.trailer()), &mut groups, &mut seen);
    groups
}

fn collect_groups<'a>(
    members: impl Iterator<Item = flat::Member<'a>>,
    groups: &mut Vec<dictionary::Group<'a>>,
    seen: &mut Vec<MemberContext>,
) {
    for member in members {
        let flat::Member::Group(group) = member else {
            continue;
        };
        let context = group.declared_in().group(&group.count_field().name);
        if seen.contains(&context) {
            continue;
        }
        seen.push(context);
        collect_groups(flat::members(group.members()), groups, seen);
        if matches!(group.declared_in(), MemberContext::Component(_)) {
            groups.push(group);
        }
    }
}

/// A group's class, which holds the class of its instances.
///
/// A group declared directly in its instances has its class in the instance
/// class, where the declaration reading it can name it. The group's class
/// names it too, so its path stays `Outer.Nested`.
pub(super) fn emit_group_class(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut items = vec![emit_instance_class(group, names)];
    let aliases: String = nested_groups(group)
        .map(|nested| {
            let class = names.group_class(nested);
            format!("{class} = Instance.{class}\n")
        })
        .collect();
    if !aliases.is_empty() {
        items.push(aliases);
    }
    format!(
        "class {}:\n{}",
        names.group_class(group),
        indent(&items.join("\n"), 1)
    )
}

fn emit_instance_class(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut items = Vec::new();
    // Python would name the class after where it is defined.
    if is_declared_in_a_group(group) {
        let path = names.group_path(group);
        items.push(format!("    __qualname__ = \"{path}.Instance\"\n"));
    }
    items.extend(nested_groups(group).map(|nested| indent(&emit_group_class(nested, names), 1)));
    items.push(emit_declarations(flat::members(group.members()), names));
    format!(
        "class Instance(fields.GroupInstance):\n{}",
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
