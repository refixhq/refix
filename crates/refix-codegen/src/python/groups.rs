use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::layout::{MAX_WIDTH, indent, tuple, tuple_lines};
use super::messages::emit_property;
use super::names::Names;
use crate::groups::{known_tags, member_tags};

/// The groups declared in components, each once, innermost first, from the
/// header, messages and trailer.
///
/// A class is defined after the classes its table names. Every other
/// group's class sits where the group is declared.
pub(super) fn shared_groups(dictionary: &Dictionary) -> Vec<dictionary::Group<'_>> {
    let mut groups = Vec::new();
    let mut seen = Vec::new();
    collect_groups(dictionary.header(), &mut groups, &mut seen);
    for message in dictionary.messages() {
        collect_groups(message.members(), &mut groups, &mut seen);
    }
    collect_groups(dictionary.trailer(), &mut groups, &mut seen);
    groups
}

fn collect_groups<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    groups: &mut Vec<dictionary::Group<'a>>,
    seen: &mut Vec<MemberContext>,
) {
    for member in members {
        let dictionary::Member::Group(group) = member else {
            continue;
        };
        let context = group.declared_in().group(&group.count_field().name);
        if seen.contains(&context) {
            continue;
        }
        seen.push(context);
        collect_groups(group.members(), groups, seen);
        if matches!(group.declared_in(), MemberContext::Component(_)) {
            groups.push(group);
        }
    }
}

/// A group's class, indented `depth` levels where it is emitted.
///
/// The classes of the groups declared directly in its instances come first
/// in it, as its table names them.
pub(super) fn emit_group_class(
    group: dictionary::Group<'_>,
    depth: usize,
    names: &Names,
) -> String {
    let width = MAX_WIDTH - 4 * (depth + 1);
    let path = names.group_path(group);
    let mut items: Vec<String> = group
        .members()
        .filter_map(|member| match member {
            dictionary::Member::Group(nested) if is_declared_in_a_group(nested) => {
                Some(emit_group_class(nested, depth + 1, names))
            }
            _ => None,
        })
        .collect();
    items.extend([
        emit_table(group, width, names),
        tuple_lines(
            "KNOWN_TAGS: ClassVar[KnownTags] = KnownTags(",
            &tags(&known_tags(group.members())),
            ")",
            width,
        ),
        emit_instance_class(group, &path, names),
    ]);
    format!(
        "class {}:\n{}",
        names.group_class(group),
        indent(&items.join("\n"), 1)
    )
}

fn emit_instance_class(group: dictionary::Group<'_>, path: &str, names: &Names) -> String {
    let mut members = vec![
        "    def __init__(self, scope: refix.Scope) -> None:\n        self._scope = scope\n"
            .to_owned(),
        format!(
            "    def __repr__(self) -> str:\n        return f\"{path}.Instance({{self._scope}})\"\n"
        ),
        "    @property\n    def raw(self) -> refix.Scope:\n        return self._scope\n".to_owned(),
    ];
    let known_tags = format!("{path}.KNOWN_TAGS");
    for member in group.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_property(field, names, "self._scope"));
            }
            dictionary::Member::Group(nested) => {
                members.push(emit_group_property(
                    nested,
                    names,
                    "self._scope",
                    &known_tags,
                ));
            }
        }
    }
    format!("class Instance:\n{}", members.join("\n"))
}

/// The property reading a group's instances from `receiver`, walking with
/// `known_tags`.
pub(super) fn emit_group_property(
    group: dictionary::Group<'_>,
    names: &Names,
    receiver: &str,
    known_tags: &str,
) -> String {
    let name = names.group_property(group);
    let path = names.group_path(group);
    let count_field = group.count_field();
    let (count_name, count_tag) = (&count_field.name, count_field.tag);
    format!(
        "    @cached_property\n    def {name}(self) -> tuple[{path}.Instance, ...]:\n        \"\"\"The instances of the {count_name} ({count_tag}) group.\"\"\"\n        scopes = {receiver}.get_group({path}.TABLE, {known_tags})\n        return () if scopes is None else tuple({path}.Instance(scope) for scope in scopes)\n"
    )
}

fn emit_table(group: dictionary::Group<'_>, width: usize, names: &Names) -> String {
    let (count_tag, delimiter, members, nested) = table_arguments(group, names);
    let head = "TABLE: ClassVar[GroupTable] = GroupTable(";
    let one_line = format!(
        "{head}{count_tag}, {delimiter}, {}, {})",
        tuple(&members),
        tuple(&nested)
    );
    if one_line.len() <= width {
        return format!("{one_line}\n");
    }
    let arguments = [
        format!("{count_tag},\n"),
        format!("{delimiter},\n"),
        tuple_lines("", &members, ",", width - 4),
        tuple_lines("", &nested, ",", width - 4),
    ];
    format!("{head}\n{})\n", indent(&arguments.concat(), 1))
}

/// The arguments of a group's `GroupTable`.
///
/// These are its count tag, its delimiter, its member tags and the tables of
/// its nested groups.
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
    (
        group.count_field().tag,
        group.delimiter().tag,
        tags(&member_tags(group)),
        nested,
    )
}

/// A nested group's `TABLE`.
///
/// A group declared directly in the instances is named without a path, as
/// its class is defined in the body the table is written in.
fn table_reference(group: dictionary::Group<'_>, names: &Names) -> String {
    if is_declared_in_a_group(group) {
        return format!("{}.TABLE", names.group_class(group));
    }
    format!("{}.TABLE", names.group_path(group))
}

fn is_declared_in_a_group(group: dictionary::Group<'_>) -> bool {
    matches!(group.declared_in(), MemberContext::Group { .. })
}

pub(super) fn tags(tags: &[Tag]) -> Vec<String> {
    tags.iter().map(ToString::to_string).collect()
}
