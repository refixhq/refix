use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::docs::{code, member_docs};
use super::layout::{MAX_WIDTH, indent, slice};
use super::messages::{Lifetime, emit_accessors};
use super::names::Names;
use super::printing::emit_debug;
use crate::envelope::Section;
use crate::flat;
use crate::groups::member_tags;

/// The groups declared in components, each once, in order of first
/// appearance across the header, messages and trailer.
///
/// Every other group's type follows whatever declares the group.
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

/// The type of a group's instances.
///
/// `Debug` prints the fields of the instance. The types of the groups
/// declared directly in its instances follow it.
pub(super) fn emit_group(group: dictionary::Group<'_>, names: &Names) -> String {
    let name = names.group_type(group);
    let count_field = group.count_field();
    let mut items = vec![
        format!(
            "/// An instance of the {} ({}) group.\n#[derive(Clone, Copy)]\npub struct {name}<'a>(refix_message::Scope<'a>);\n",
            code(&count_field.name),
            count_field.tag
        ),
        format!(
            "impl<'a> From<refix_message::Scope<'a>> for {name}<'a> {{\n    fn from(scope: refix_message::Scope<'a>) -> Self {{\n        Self(scope)\n    }}\n}}\n"
        ),
        emit_instance_impl(group, names),
        emit_debug(name, "self.0"),
    ];
    for member in flat::members(group.members()) {
        if let flat::Member::Group(nested) = member
            && matches!(nested.declared_in(), MemberContext::Group { .. })
        {
            items.push(emit_group(nested, names));
        }
    }
    items.join("\n")
}

/// The table an instance is walked with, its known tags, then its methods.
fn emit_instance_impl(group: dictionary::Group<'_>, names: &Names) -> String {
    let mut members = vec![
        indent(&emit_table(group, names), 1),
        indent(&emit_known_tags(group.members(), &[], names), 1),
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    ];
    members.extend(emit_accessors(
        group.members(),
        names,
        Lifetime::Scope,
        KNOWN_TAGS,
    ));
    format!(
        "impl<'a> {}<'a> {{\n{}}}\n",
        names.group_type(group),
        members.join("\n")
    )
}

/// The accessor reading a group's instances, walking with `known_tags`.
pub(super) fn emit_group_accessor(
    group: dictionary::Group<'_>,
    names: &Names,
    lifetime: Lifetime,
    known_tags: &str,
) -> String {
    let name = names.group_accessor(group);
    let type_name = names.group_type(group);
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
        "{docs}    pub fn {name}(&self) -> Result<Instances<{lifetime}, {type_name}<{lifetime}>>, InvalidValue> {{\n        self.0.get_group(&{type_name}::TABLE, &{known_tags}).map(Instances::from)\n    }}\n"
    )
}

fn emit_table(group: dictionary::Group<'_>, names: &Names) -> String {
    let width = MAX_WIDTH - 4;
    let (count_tag, delimiter, members, nested) = table_arguments(group, names);
    let head = "const TABLE: GroupTable<'static> = GroupTable::new(";
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
            flat::Member::Group(child) => Some(format!("{}::TABLE", names.group_type(child))),
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

/// The known tags of the type whose `known_tags` function is in scope.
pub(super) const KNOWN_TAGS: &str = "Self::known_tags().get()";

/// The function holding every tag a type can contain at any depth.
///
/// It writes the tags `members` list themselves. The rest come from the
/// functions of `sections` and of the components and groups among the
/// members.
pub(super) fn emit_known_tags<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    sections: &[Section],
    names: &Names,
) -> String {
    let mut own = Vec::new();
    let mut parts: Vec<String> = sections
        .iter()
        .map(|section| known_tags_of(section.type_name()))
        .collect();
    for member in members {
        match member {
            dictionary::Member::Field { field, .. } => own.push(format!("Tag({})", field.tag)),
            dictionary::Member::Group(group) => {
                own.push(format!("Tag({})", group.count_field().tag));
                parts.push(known_tags_of(names.group_type(group)));
            }
            dictionary::Member::Component(component) => {
                parts.push(known_tags_of(component.name()));
            }
        }
    }
    emit_lazy_tags("known_tags", &own, &parts)
}

/// The call returning the known tags a type holds.
pub(super) fn known_tags_of(type_name: &str) -> String {
    format!("{type_name}::known_tags()")
}

/// A function named `name` holding known tags that are merged on first use.
///
/// `own` are tags and `parts` are calls returning other such lists.
pub(super) fn emit_lazy_tags(name: &str, own: &[String], parts: &[String]) -> String {
    let head = "static TAGS: LazyKnownTags = LazyKnownTags::new(";
    let one_line = format!("{head}&[{}], &[{}]);\n", own.join(", "), parts.join(", "));
    let list = if one_line.len() <= MAX_WIDTH - 8 {
        one_line
    } else {
        let width = MAX_WIDTH - 12;
        let arguments = slice("", own, ",", width) + &slice("", parts, ",", width);
        format!("{head}\n{});\n", indent(&arguments, 1))
    };
    format!(
        "const fn {name}() -> &'static LazyKnownTags {{\n{}    &TAGS\n}}\n",
        indent(&list, 1)
    )
}
