use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::docs::{code, member_docs};
use super::layout::{MAX_WIDTH, indent, slice};
use super::messages::{Lifetime, emit_accessors};
use super::names::Names;
use crate::flat;
use crate::groups::{known_tags, member_tags};

/// The groups declared in components, each once, in order of first
/// appearance across the header, messages and trailer.
///
/// Every other group's module sits where the group is declared.
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
        if matches!(group.declared_in(), MemberContext::Component(_)) {
            groups.push(group);
        }
        collect_groups(flat::members(group.members()), groups, seen);
    }
}

/// A group's module, with the modules of the groups declared directly in
/// its instances inside it.
pub(super) fn emit_group_module(group: dictionary::Group<'_>, names: &Names) -> String {
    let depth = names.group_path(group).split("::").count();
    let width = MAX_WIDTH - 4 * depth;
    let mut items = vec![
        format!("use {}*;\n", "super::".repeat(depth)),
        emit_table(group, names, width),
        emit_known_tags(&known_tags(flat::members(group.members())), width),
        "#[derive(Clone, Copy, Debug)]\npub struct Instance<'a>(refix_message::Scope<'a>);\n".to_owned(),
        "impl<'a> From<refix_message::Scope<'a>> for Instance<'a> {\n    fn from(scope: refix_message::Scope<'a>) -> Self {\n        Self(scope)\n    }\n}\n".to_owned(),
        emit_instance_impl(group, names),
    ];
    for member in flat::members(group.members()) {
        if let flat::Member::Group(nested) = member
            && matches!(nested.declared_in(), MemberContext::Group { .. })
        {
            items.push(emit_group_module(nested, names));
        }
    }
    format!(
        "pub mod {} {{\n{}}}\n",
        names.group_module(group),
        indent(&items.join("\n"), 1)
    )
}

fn emit_instance_impl(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut members = vec![
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    ];
    members.extend(emit_accessors(
        group.members(),
        names,
        Lifetime::Scope,
        "KNOWN_TAGS",
        true,
    ));
    format!("impl<'a> Instance<'a> {{\n{}}}\n", members.join("\n"))
}

/// The accessor reading a group's instances, walking with `known_tags`.
pub(super) fn emit_group_accessor(
    group: dictionary::Group<'_>,
    names: &Names,
    lifetime: Lifetime,
    known_tags: &str,
) -> String {
    let name = names.group_accessor(group);
    let path = names.group_path(group);
    let count_field = group.count_field();
    let (count_name, count_tag) = (&count_field.name, count_field.tag);
    let lifetime = match lifetime {
        Lifetime::Receiver => "'_",
        Lifetime::Scope => "'a",
    };
    let summary = format!(
        "The instances of the {} ({count_tag}) group.",
        code(count_name)
    );
    let docs = member_docs(&summary, count_name, name);
    format!(
        "{docs}    pub fn {name}(&self) -> Result<Instances<{lifetime}, {path}::Instance<{lifetime}>>, InvalidValue> {{\n        self.0.get_group(&{path}::TABLE, &{known_tags}).map(Instances::from)\n    }}\n"
    )
}

fn emit_table(group: dictionary::Group<'_>, names: &Names, width: usize) -> String {
    let (count_tag, delimiter, members, nested) = table_arguments(group, names);
    let head = "pub const TABLE: GroupTable<'static> = GroupTable::new(";
    let one_line = format!(
        "{head}Tag({count_tag}), Tag({delimiter}), &[{}], &[{}]);",
        members.join(", "),
        nested.join(", ")
    );
    if one_line.len() <= width {
        return format!("{one_line}\n");
    }
    let arguments = [
        format!("Tag({count_tag}),\n"),
        format!("Tag({delimiter}),\n"),
        slice("", &members, ",", width - 4),
        slice("", &nested, ",", width - 4),
    ];
    format!("{head}\n{});\n", indent(&arguments.concat(), 1))
}

/// The arguments of a group's `GroupTable::new`.
///
/// These are the count tag, the delimiter, the tags its instances directly
/// contain and the tables of its nested groups.
fn table_arguments(
    group: dictionary::Group<'_>,
    names: &Names,
) -> (Tag, Tag, Vec<String>, Vec<String>) {
    let nested = flat::members(group.members())
        .filter_map(|member| match member {
            flat::Member::Group(child) => Some(format!("{}::TABLE", names.group_path(child))),
            flat::Member::Field { .. } => None,
        })
        .collect();
    let members = member_tags(group)
        .iter()
        .map(|tag| format!("Tag({tag})"))
        .collect();
    (
        group.count_field().tag,
        group.delimiter().tag,
        members,
        nested,
    )
}

/// The `KNOWN_TAGS` constant listing `tags`.
pub(super) fn emit_known_tags(tags: &[Tag], width: usize) -> String {
    let tags: Vec<String> = tags.iter().map(|tag| format!("Tag({tag})")).collect();
    slice(
        "pub const KNOWN_TAGS: KnownTags<'static> = KnownTags::new(",
        &tags,
        ");",
        width,
    )
}
