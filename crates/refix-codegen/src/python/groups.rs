use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::layout::{MAX_WIDTH, indent, tuple, tuple_lines};
use super::messages::emit_property;
use super::naming::group_property_name;
use crate::Warning;
use crate::groups::{is_generated, known_tags, member_tags};
use crate::naming::group_source_name;

/// The groups declared in components, each once, innermost first.
///
/// A class is defined after the classes its table names. A group declared
/// directly in another group's instance gets no class and is reported
/// instead.
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
        collect_groups(group.members(), groups, seen, warnings);
        match group.declared_in() {
            MemberContext::Component(_) => groups.push(group),
            MemberContext::Message(_) => {}
            MemberContext::Group { .. } => warnings.push(Warning::UnsupportedGroup { context }),
        }
    }
}

/// The path of a group's class from the module: a shared group's own name,
/// or a message's group under the message's class.
fn group_path(group: dictionary::Group<'_>) -> String {
    let name = group_source_name(group);
    match group.declared_in() {
        MemberContext::Message(message) => format!("{message}.{name}"),
        _ => name.to_owned(),
    }
}

/// A group's class, indented `depth` levels where it is emitted.
pub(super) fn emit_group_class(group: dictionary::Group<'_>, depth: usize) -> String {
    let width = MAX_WIDTH - 4 * (depth + 1);
    let path = group_path(group);
    let items = [
        emit_table(group, width),
        tuple_lines(
            "KNOWN_TAGS: ClassVar[KnownTags] = KnownTags(",
            &tags(&known_tags(group.members())),
            ")",
            width,
        ),
        emit_instance_class(group, &path),
    ];
    format!(
        "class {}:\n{}",
        group_source_name(group),
        indent(&items.join("\n"), 1)
    )
}

fn emit_instance_class(group: dictionary::Group<'_>, path: &str) -> String {
    let mut members = vec![
        "    def __init__(self, scope: refix.Scope) -> None:\n        self._scope = scope\n"
            .to_owned(),
        "    @property\n    def raw(self) -> refix.Scope:\n        return self._scope\n".to_owned(),
    ];
    let known_tags = format!("{path}.KNOWN_TAGS");
    for member in group.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_property(field, "self._scope"));
            }
            dictionary::Member::Group(nested) if is_generated(nested) => {
                members.push(emit_group_property(nested, "self._scope", &known_tags));
            }
            dictionary::Member::Group(_) => {}
        }
    }
    format!("class Instance:\n{}", members.join("\n"))
}

/// The property reading a group's instances from `receiver`, walking with
/// `known_tags`.
pub(super) fn emit_group_property(
    group: dictionary::Group<'_>,
    receiver: &str,
    known_tags: &str,
) -> String {
    let name = group_property_name(group);
    let path = group_path(group);
    format!(
        "    @cached_property\n    def {name}(self) -> tuple[{path}.Instance, ...]:\n        scopes = {receiver}.get_group({path}.TABLE, {known_tags})\n        return () if scopes is None else tuple({path}.Instance(scope) for scope in scopes)\n"
    )
}

fn emit_table(group: dictionary::Group<'_>, width: usize) -> String {
    let (count_tag, delimiter, members, nested) = table_arguments(group);
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

/// The arguments of a group's `GroupTable`: its count tag, its delimiter,
/// its member tags and the tables of its nested groups.
fn table_arguments(group: dictionary::Group<'_>) -> (Tag, Tag, Vec<String>, Vec<String>) {
    let nested = group
        .members()
        .filter_map(|member| match member {
            dictionary::Member::Group(child) => Some(table_reference(child)),
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

/// A nested group's `TABLE`, written out in place for a group without a class.
fn table_reference(group: dictionary::Group<'_>) -> String {
    if is_generated(group) {
        return format!("{}.TABLE", group_path(group));
    }
    let (count_tag, delimiter, members, nested) = table_arguments(group);
    format!(
        "GroupTable({count_tag}, {delimiter}, {}, {})",
        tuple(&members),
        tuple(&nested)
    )
}

pub(super) fn tags(tags: &[Tag]) -> Vec<String> {
    tags.iter().map(ToString::to_string).collect()
}
