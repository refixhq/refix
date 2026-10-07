use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::layout::{MAX_WIDTH, indent, slice};
use super::messages::{Lifetime, emit_accessor};
use super::names::Names;
use crate::Warning;
use crate::groups::{is_generated, known_tags, member_tags};

/// The groups declared in components, each once, in order of first
/// appearance across the messages.
///
/// A group declared directly in another group's instance gets no module
/// and is reported instead.
pub(super) fn shared_groups<'a>(
    dictionary: &'a Dictionary,
    warnings: &mut Vec<Warning>,
) -> Vec<dictionary::Group<'a>> {
    let mut groups = Vec::new();
    let mut seen = Vec::new();
    for message in dictionary.messages() {
        collect_groups(message.members(), &mut groups, &mut seen, warnings);
    }
    groups
}

fn collect_groups<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    groups: &mut Vec<dictionary::Group<'a>>,
    seen: &mut Vec<MemberContext>,
    warnings: &mut Vec<Warning>,
) {
    for member in members {
        let dictionary::Member::Group(group) = member else {
            continue;
        };
        let context = group.declared_in().group(&group.count_field().name);
        if seen.contains(&context) {
            continue;
        }
        seen.push(context.clone());
        match group.declared_in() {
            MemberContext::Component(_) => groups.push(group),
            MemberContext::Message(_) | MemberContext::Header | MemberContext::Trailer => {}
            MemberContext::Group { .. } => warnings.push(Warning::UnsupportedGroup { context }),
        }
        collect_groups(group.members(), groups, seen, warnings);
    }
}

pub(super) fn emit_group_module(group: dictionary::Group<'_>, names: &Names) -> String {
    let depth = names.group_path(group).split("::").count();
    let width = MAX_WIDTH - 4 * depth;
    let items = [
        format!("use {}*;\n", "super::".repeat(depth)),
        emit_table(group, names, width),
        emit_known_tags(&known_tags(group.members()), width),
        "#[derive(Clone, Copy, Debug)]\npub struct Instance<'a>(refix_message::Scope<'a>);\n".to_owned(),
        "impl<'a> From<refix_message::Scope<'a>> for Instance<'a> {\n    fn from(scope: refix_message::Scope<'a>) -> Self {\n        Self(scope)\n    }\n}\n".to_owned(),
        emit_instance_impl(group, names),
    ];
    format!(
        "pub mod {} {{\n{}}}\n",
        names.group(group),
        indent(&items.join("\n"), 1)
    )
}

fn emit_instance_impl(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut members = vec![
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    ];
    for member in group.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_accessor(field, names, Lifetime::Scope))
            }
            dictionary::Member::Group(nested) if is_generated(nested) => members.push(
                emit_group_accessor(nested, names, Lifetime::Scope, "KNOWN_TAGS"),
            ),
            dictionary::Member::Group(_) => {}
        }
    }
    format!("impl<'a> Instance<'a> {{\n{}}}\n", members.join("\n"))
}

/// The accessor reading a group's instances, walking with `known_tags`.
pub(super) fn emit_group_accessor(
    group: dictionary::Group<'_>,
    names: &Names,
    lifetime: Lifetime,
    known_tags: &str,
) -> String {
    let name = names.group(group);
    let path = names.group_path(group);
    let lifetime = match lifetime {
        Lifetime::Receiver => "'_",
        Lifetime::Scope => "'a",
    };
    format!(
        "    pub fn {name}(&self) -> Result<Instances<{lifetime}, {path}::Instance<{lifetime}>>, InvalidValue> {{\n        self.0.get_group(&{path}::TABLE, &{known_tags}).map(Instances::from)\n    }}\n"
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
    let nested = group
        .members()
        .filter_map(|member| match member {
            dictionary::Member::Group(child) => Some(table_reference(child, names)),
            dictionary::Member::Field { .. } => None,
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

/// A nested group's `TABLE`, written out in place for a group without a module.
fn table_reference(group: dictionary::Group<'_>, names: &Names) -> String {
    if is_generated(group) {
        return format!("{}::TABLE", names.group_path(group));
    }
    let (count_tag, delimiter, members, nested) = table_arguments(group, names);
    format!(
        "GroupTable::new(Tag({count_tag}), Tag({delimiter}), &[{}], &[{}])",
        members.join(", "),
        nested.join(", ")
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
